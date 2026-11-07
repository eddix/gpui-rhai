//! Files and folders the operating system drags onto a DropZone that lists `os.files`
//! (#28): only when the Host accepts file drops, highlighted while they hover, with an
//! opaque handle per item that only the Host turns back into a path, and limited to the
//! zone's `file_kinds`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{ExternalPaths, FileDropEvent, TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, ReplaceOnce, Step, bounds, center, filled, settle, text_of};

const ACCENT: u32 = 0x1234_56ff;
const HOVER_FILL: u32 = 0x1234_5628;

const SCRIPT: &str = r#"
import "components/drop_zone" as drop_zone;
fn state_schema() { #{ fields: #{ status: #{ schema: #{ type: "string" }, "default": "none" } } } }
fn dropped(ctx, value) {
    let files = [];
    for file in value.payload.files {
        files.push(`${file.name}:${file.kind}:${file.size}`);
        ctx.call_capability("app.files", "open", file.file);
    }
    ctx.set_state("status", `${value.source_id} ${value.payload_type} ${files}`);
}
fn view(ctx) {
    column([
        text(ctx.get_state("status")).test_id("status"),
        drop_zone::DropZone(#{ key: "files", label: "Files", target_id: "files",
            payload_types: ["os.files"], operations: ["copy"], FILE_KINDS on_drop: Fn("dropped"),
            content: text("Drop files").with_style(style().width(px(200)).height(px(100))) }),
    ])
}
"#;

/// The handles the script passes to `app.files.open`, with the path the handler
/// resolved for each.
#[derive(Clone, Default)]
struct Opened(Rc<RefCell<Vec<OpenCall>>>);

type OpenCall = (OpaqueHandle, Option<PathBuf>);

/// The Host's capability: it turns a handle into the path with the runtime's
/// `dropped_files`, which its extension cloned.
struct Open {
    dropped: DroppedFiles,
    opened: Opened,
}

impl CapabilityHandler for Open {
    fn call(&mut self, _: &str, input: UiValue) -> Result<UiValue, String> {
        let UiValue::Handle(handle) = input else {
            return Err("open takes a handle".into());
        };
        let path = self.dropped.path(&handle);
        self.opened.0.borrow_mut().push((handle, path));
        Ok(UiValue::Null)
    }
}

impl ScriptViewExtension for Opened {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        let open = Open {
            dropped: runtime.dropped_files.clone(),
            opened: self.clone(),
        };
        runtime
            .capabilities
            .register(
                CapabilityDescriptor {
                    id: CapabilityId::parse("app.files").unwrap(),
                    version: semver::Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "open".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::Handle {
                                kind: "file".to_owned(),
                            },
                            output: ValueSchema::Null,
                        },
                    )]),
                },
                open,
            )
            .map_err(|error| error.to_string())
    }
}

fn mount(
    cx: &mut TestAppContext,
    file_kinds: &str,
    accept: bool,
) -> (VisualTestContext, ScriptViewHandle, Opened) {
    let opened = Opened::default();
    let extension = opened.clone();
    let manifest = AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("app.files", "*")
        .unwrap();
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("accent".to_owned(), Rgba8::from_rgba_hex(ACCENT))]),
        ..Default::default()
    };
    let (mut visual, view) = Mount::new(SCRIPT.replace_once("FILE_KINDS", file_kinds))
        .registry("components/drop_zone")
        .motion(MotionPreference::None)
        .with(move |view| {
            view.manifest(manifest)
                .theme_token_overrides(overrides)
                .accept_file_drops(accept)
                .extension(extension)
        })
        .id("file-drops")
        .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    (visual, view, opened)
}

/// A temporary folder, removed on drop.
struct Folder(PathBuf);

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A folder holding `notes.txt` (5 bytes) and the folder `photos`.
fn fixture(name: &str) -> (Folder, PathBuf, PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "gpui-rhai-file-drops-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("notes.txt");
    std::fs::write(&file, "hello").unwrap();
    let folder = directory.join("photos");
    std::fs::create_dir(&folder).unwrap();
    (Folder(directory), file, folder)
}

fn drag_over(visual: &mut VisualTestContext, view: &ScriptViewHandle, paths: &[&Path]) {
    let position = center(bounds(visual, view, "group", "Files"));
    visual.simulate_event(FileDropEvent::Entered {
        position,
        paths: ExternalPaths(paths.iter().map(|path| path.to_path_buf()).collect()),
    });
    visual.simulate_event(FileDropEvent::Pending { position });
    settle(visual, Step::Frame, 1);
}

fn drop_here(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    let position = center(bounds(visual, view, "group", "Files"));
    visual.simulate_event(FileDropEvent::Submit { position });
    visual.simulate_event(FileDropEvent::Ended);
    settle(visual, Step::Frame, 2);
}

#[gpui::test]
fn a_file_drop_gives_the_script_handles_and_the_host_paths(cx: &mut TestAppContext) {
    let (_directory, file, folder) = fixture("handles");
    let (mut visual, view, opened) = mount(cx, "", true);

    drag_over(&mut visual, &view, &[&file, &folder]);
    assert!(
        !filled(&mut visual, HOVER_FILL).is_empty(),
        "the zone highlights while files hover over it"
    );
    drop_here(&mut visual, &view);
    assert!(filled(&mut visual, HOVER_FILL).is_empty());
    assert_eq!(
        text_of(&mut visual, &view, "status"),
        r#"os os.files ["notes.txt:file:5", "photos:dir:"]"#
    );

    // Only the Host learns the paths, through the handles the script passed on.
    let opened = opened.0.borrow().clone();
    assert_eq!(
        opened
            .iter()
            .map(|(_, path)| path.clone())
            .collect::<Vec<_>>(),
        [Some(file.clone()), Some(folder)]
    );
    visual.update(|_, cx| {
        assert_eq!(
            view.dropped_file_path(&opened[0].0, cx).unwrap(),
            Some(file)
        );
        for handle in [OpaqueHandle::new("file", 9), OpaqueHandle::new("image", 0)] {
            assert_eq!(view.dropped_file_path(&handle, cx).unwrap(), None);
        }
    });
}

#[gpui::test]
fn file_drops_are_off_unless_the_host_accepts_them(cx: &mut TestAppContext) {
    let (_directory, file, _) = fixture("off");
    let (mut visual, view, opened) = mount(cx, "", false);
    drag_over(&mut visual, &view, &[&file]);
    assert!(filled(&mut visual, HOVER_FILL).is_empty());
    drop_here(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "status"), "none");
    assert!(opened.0.borrow().is_empty());
}

#[gpui::test]
fn file_kinds_limit_what_a_zone_takes(cx: &mut TestAppContext) {
    let (_directory, file, folder) = fixture("kinds");
    let (mut visual, view, _) = mount(cx, r#"file_kinds: ["file"],"#, true);

    drag_over(&mut visual, &view, &[&file, &folder]);
    assert!(filled(&mut visual, HOVER_FILL).is_empty());
    drop_here(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "status"), "none");

    drag_over(&mut visual, &view, &[&file]);
    drop_here(&mut visual, &view);
    assert_eq!(
        text_of(&mut visual, &view, "status"),
        r#"os os.files ["notes.txt:file:5"]"#
    );
}
