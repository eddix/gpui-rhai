//! The capability recorder of the theme appearance probes: scripts call
//! `<capability>.record(text)` and the probe reads what was recorded.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_rhai::{
    CapabilityDescriptor, CapabilityHandler, CapabilityId, CapabilityMethod, ScriptViewExtension,
    ThemePreference, UiRuntimeState, UiValue, ValueSchema,
};

/// The recorded texts, and an extension that registers the `record` capability.
#[derive(Clone)]
pub struct Records {
    pub values: Rc<RefCell<Vec<String>>>,
    capability: &'static str,
    /// Whether the view's theme follows the system appearance.
    system: bool,
}

impl Records {
    /// An empty record behind `capability`, for views that follow the system appearance.
    pub fn new(capability: &'static str) -> Self {
        Self {
            values: Rc::new(RefCell::new(Vec::new())),
            capability,
            system: true,
        }
    }

    /// The same record, for a view that follows the system appearance or keeps its theme.
    pub fn following_system(&self, system: bool) -> Self {
        Self {
            system,
            ..self.clone()
        }
    }
}

impl CapabilityHandler for Records {
    fn call(&mut self, _: &str, input: UiValue) -> Result<UiValue, String> {
        let UiValue::String(value) = input else {
            return Err("record needs a string".into());
        };
        self.values.borrow_mut().push(value);
        Ok(UiValue::Null)
    }
}

impl ScriptViewExtension for Records {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        if self.system {
            runtime
                .theme
                .as_mut()
                .ok_or("theme manager missing")?
                .set_app(ThemePreference::System {
                    family: "Default".into(),
                })
                .map_err(|error| error.to_string())?;
        }
        runtime
            .capabilities
            .register(
                CapabilityDescriptor {
                    id: CapabilityId::parse(self.capability).unwrap(),
                    version: semver::Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "record".into(),
                        CapabilityMethod {
                            input: ValueSchema::string(),
                            output: ValueSchema::Null,
                        },
                    )]),
                },
                self.clone(),
            )
            .map_err(|error| error.to_string())
    }
}
