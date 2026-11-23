// Gallery spec pages: fields and choices.

import "gallery/kit" as kit;
import "components/input" as input;
import "components/number_input" as number_input;
import "components/otp_input" as otp_input;
import "components/tag_input" as tag_input;
import "components/filter_menu" as filter_menu;
import "components/stepper_form" as stepper_form;
import "components/selectable_card_group" as selectable_card_group;
import "components/textarea" as textarea;
import "components/input_group" as input_group;
import "components/select" as select;
import "components/combobox" as combobox;
import "components/date_picker" as date_picker;
import "components/checkbox" as checkbox;
import "components/radio" as radio;
import "components/radio_group" as radio_group;
import "components/switch" as switch_control;
import "components/slider" as slider;
import "components/range_slider" as range_slider;
import "components/label" as label;
import "components/form_field" as form_field;
import "components/button" as button;

fn pages() {
    [
        #{ id: "input", title: "Input", group: "Fields" },
        #{ id: "number_input", title: "NumberInput", group: "Fields" },
        #{ id: "otp_input", title: "OtpInput", group: "Fields" },
        #{ id: "tag_input", title: "TagInput", group: "Fields" },
        #{ id: "filter_menu", title: "FilterMenu", group: "Fields" },
        #{ id: "stepper_form", title: "StepperForm", group: "Fields" },
        #{ id: "textarea", title: "Textarea", group: "Fields" },
        #{ id: "input_group", title: "InputGroup", group: "Fields" },
        #{ id: "select", title: "Select", group: "Fields" },
        #{ id: "combobox", title: "Combobox", group: "Fields" },
        #{ id: "date_picker", title: "DatePicker", group: "Fields" },
        #{ id: "checkbox", title: "Checkbox", group: "Fields" },
        #{ id: "radio", title: "Radio", group: "Fields" },
        #{ id: "radio_group", title: "RadioGroup", group: "Fields" },
        #{ id: "switch", title: "Switch", group: "Fields" },
        #{ id: "slider", title: "Slider", group: "Fields" },
        #{ id: "range_slider", title: "RangeSlider", group: "Fields" },
        #{ id: "label", title: "Label", group: "Fields" },
        #{ id: "form_field", title: "FormField", group: "Fields" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "input" { input_page(h, v) }
    else if id == "number_input" { number_input_page(h, v) }
    else if id == "otp_input" { otp_input_page(h, v) }
    else if id == "tag_input" { tag_input_page(h, v) }
    else if id == "filter_menu" { filter_menu_page(h, v) }
    else if id == "stepper_form" { stepper_form_page(h, v) }
    else if id == "textarea" { textarea_page(h, v) }
    else if id == "input_group" { input_group_page(h, v) }
    else if id == "select" { select_page(h, v) }
    else if id == "combobox" { combobox_page(h, v) }
    else if id == "date_picker" { date_picker_page(h, v) }
    else if id == "checkbox" { checkbox_page(h, v) }
    else if id == "radio" { radio_page(h, v) }
    else if id == "radio_group" { radio_group_page(h, v) }
    else if id == "switch" { switch_page(h, v) }
    else if id == "slider" { slider_page(h, v) }
    else if id == "range_slider" { range_slider_page(h, v) }
    else if id == "label" { label_page(h, v) }
    else if id == "form_field" { form_field_page(h, v) }
    else { () }
}

fn wizard_submit(ctx,request){
    let values=ctx.get_state("values");
    values["wizard.request"]=request.request;
    values["wizard.status"]="submitting";
    ctx.set_state("values",values);
}
fn wizard_saved(ctx,request){
    let values=ctx.get_state("values");
    if kit::val(values,"wizard.request",0)==request&&kit::val(values,"wizard.status","editing")=="submitting"{
        values["wizard.status"]="succeeded";ctx.set_state("values",values);
    }
}
fn wizard_reset(ctx,payload){
    let values=ctx.get_state("values");
    for key in values.keys(){if key.starts_with("wizard."){values.remove(key);}}
    ctx.set_state("values",values);
}
fn stepper_form_page(h,v){
    let name=kit::val(v,"wizard.name","");let trimmed=name;trimmed.trim();
    let error=if trimmed==""{"Enter a workspace name."}else{()};
    let name_error=if kit::val(v,"wizard.attempted",false){error}else{()};
    let tags=kit::val(v,"wizard.team",[]);
    let plan=kit::val(v,"wizard.plan",["standard"]);
    let status=kit::val(v,"wizard.status","editing");
    if status=="submitting"{timeout("wizard-save",650,false,Fn("wizard_saved"),kit::val(v,"wizard.request",0));}
    let workspace=form_field::FormField(#{id:"wizard-name",label:"Workspace name",required:true,error:name_error,
        control:input::Input(#{key:"workspace-name",label:"Workspace name",value:name,placeholder:"e.g. Design Systems",
            error:name_error!=(),on_change:kit::store(h,"wizard.name")})});
    let team=tag_input::TagInput(#{key:"wizard-recipients",label:"Invite teammates",values:tags,optional:true,avatar:true,
        normalize:"lowercase",validation:"email",max:8,on_change:kit::store(h,"wizard.team")});
    let cards=selectable_card_group::SelectableCardGroup(#{key:"wizard-plan",label:"Plan",layout:"column",values:plan,
        items:[
            #{value:"standard",label:"Standard",content:text("One shared workspace.")},
            #{value:"team",label:"Team",content:text("More collaboration and review tools.")},
        ],on_change:kit::store(h,"wizard.plan")});
    let form=stepper_form::StepperForm(#{key:"workspace-setup",status:status,steps:[
        #{id:"account",title:"Workspace",heading:"Name your workspace",description:"Give the team a name it will recognize.",content:workspace,error:error},
        #{id:"team",title:"Team",heading:"Invite your team",description:"Optional. You can do this later.",content:team},
        #{id:"plan",title:"Plan",heading:"Choose a plan",content:cards},
        #{id:"review",title:"Review",heading:"Review your workspace",review:[
            #{key:"name",label:"Workspace name",text:name,edit_step:"account"},
            #{key:"team",label:"Teammates",text:`${tags.len} invited`,edit_step:"team"},
            #{key:"plan",label:"Plan",text:if "team" in plan{"Team"}else{"Standard"},edit_step:"plan"},
        ]}
    ],on_invalid:kit::put(h,"wizard.attempted",true),on_submit:Fn("wizard_submit"),on_reset:Fn("wizard_reset")});
    kit::page(h,v,"stepper_form","StepperForm","Validate, go back, edit from review and submit a real retained workflow.",[
        kit::sec("Set up a workspace",form),
        kit::note("This demo saves locally; no network request is made."),
    ])
}
fn filter_menu_page(h,v){
    let values=kit::val(v,"filters.applied",#{status:["active","review"],region:["apac","eu"],access:["internal"]});
    let groups=[
        #{id:"status",label:"Status",options:[#{id:"active",label:"Active"},#{id:"review",label:"In review"},
            #{id:"archived",label:"Archived"}]},
        #{id:"region",label:"Region",chip_prefix:"Region: ",options:[#{id:"apac",label:"Asia Pacific"},
            #{id:"eu",label:"Europe"},#{id:"americas",label:"Americas"}]},
        #{id:"access",label:"Access",options:[#{id:"internal",label:"Internal only"},#{id:"external",label:"External"}]},
    ];
    let total=0;for key in values.keys(){total+=values[key].len;}
    kit::page(h,v,"filter_menu","FilterMenu","Grouped choices, removable chips and one keyboard route.",[
        kit::sec("Deployment filters",filter_menu::FilterMenu(#{key:"deployment-filters",title:"Filters",groups:groups,value:values,
            on_change:kit::store(h,"filters.applied")})),
        kit::note(`Applied selections: ${total}`),
    ])
}
fn tag_section(title, field){
    // The field owns its inset label and hint. They need not match the
    // surrounding specimen heading exactly at fractional reading scales.
    kit::sec(title,field).audit_allow(["text-edge-misaligned"])
}
fn tag_input_page(h,v){
    kit::page(h,v,"tag_input","TagInput","An ordered set in one native editor, with batch paste and keyboard chip navigation.",[
        tag_section("Categories",tag_input::TagInput(#{key:"project-labels",label:"Project labels",optional:true,
            values:kit::val(v,"tag_input.labels",["Design","Documentation","待审阅"]),max:8,
            hint:"Type a label, then press Enter or comma. Backspace twice on an empty field removes the last tag.",
            on_change:kit::store(h,"tag_input.labels")
        }).with_style(style().width(px(500)).max_width(relative(1.0)))),
        tag_section("Recipients",tag_input::TagInput(#{key:"recipients",label:"Recipients",avatar:true,max:5,
            values:kit::val(v,"tag_input.recipients",["aya@example.org","lin@example.org"]),normalize:"lowercase",validation:"email",split_on_space:true,
            hint:"Paste a list of email addresses. Invalid addresses stay in the editor.",on_change:kit::store(h,"tag_input.recipients")
        }).with_style(style().width(px(500)).max_width(relative(1.0)))),
        tag_section("Disabled",tag_input::TagInput(#{key:"disabled-tags",label:"Synced labels",values:["Archived"],disabled:true})
            .with_style(style().width(px(500)).max_width(relative(1.0)))),
    ])
}
fn field(h, v, key, label_text, placeholder) {
    input::Input(#{ key: key, label: label_text, value: kit::val(v, key, ""), placeholder: placeholder,
        on_change: kit::store(h, key) }).with_style(style().width(px(200)))
}

fn otp_input_page(h,v){
    let live=kit::val(v,"otp.value","");
    let status=kit::val(v,"otp.status","idle");
    kit::page(h,v,"otp_input","OtpInput","One native editor with distinct cells. Type or paste a code, or inspect a verification result.",[
        kit::sec("Verification",kit::pile([
            otp_input::OtpInput(#{key:"verification",label:"Verification code",value:live,
                length:6,separator:true,status:status,on_change:kit::store(h,"otp.value"),
                on_complete:kit::put(h,"otp.status","success"),resend_seconds:0,
                on_resend:kit::put(h,"otp.status","idle")}).with_style(style().width(px(360))),
            kit::line([
                button::Button(#{text:"Edit",variant:"ghost",on_click:kit::put(h,"otp.status","idle")}),
                button::Button(#{text:"Verifying",variant:"ghost",on_click:kit::put(h,"otp.status","verifying")}),
                button::Button(#{text:"Reject",variant:"ghost",on_click:kit::put(h,"otp.status","error")}),
            ]),
        ]).with_style(style().gap(theme_spacing("md")))),
        kit::sec("Code variants",kit::line_top([
            otp_input::OtpInput(#{key:"recovery",label:"Recovery code",value:kit::val(v,"otp.recovery","A3"),
                length:4,alphanumeric:true,on_change:kit::store(h,"otp.recovery")})
                .with_style(style().width(px(230))),
            otp_input::OtpInput(#{key:"locked",label:"Disabled code",value:"1234",length:4,disabled:true})
                .with_style(style().width(px(230))),
        ])),
    ])
}

fn number_input_page(h,v) {
    let budget=kit::val(v,"number.budget",4000);
    let quota=kit::val(v,"number.quota",25);
    kit::page(h,v,"number_input","NumberInput",
        "Edit a value and press Enter or leave the field. Use arrows or the step buttons to adjust it.",[
        kit::sec("Numeric settings",kit::line_top([
            number_input::NumberInput(#{key:"budget",label:"Token budget",value:budget,min:0,max:8000,
                step:100,large_step:1000,unit:"tokens",hint:"Use Page Up or Page Down for larger steps.",
                on_change:kit::store(h,"number.budget")}).with_style(style().width(px(270))),
            number_input::NumberInput(#{key:"quota",label:"Usage limit",value:quota,min:0,max:100,
                step:0.5,unit:"%",optional:true,on_change:kit::store(h,"number.quota")})
                .with_style(style().width(px(200))),
        ])),
        kit::sec("Field states",kit::line_top([
            number_input::NumberInput(#{key:"missing",label:"Retry limit",value:kit::val(v,"number.retry",()),
                min:0,max:10,placeholder:"No limit",error:"Enter a limit from 0 to 10.",
                on_change:kit::store(h,"number.retry")}).with_style(style().width(px(220))),
            number_input::NumberInput(#{key:"read-only",label:"Current price",value:20.75,prefix:text("$"),
                read_only:true}).with_style(style().width(px(190))),
            number_input::NumberInput(#{key:"disabled",label:"Hard limit",value:1000,disabled:true})
                .with_style(style().width(px(190))),
        ])),
    ])
}

fn input_sizes(h, v, suffix) {
    let fields = [];
    for item in [#{size:"xs",label:"Extra small"},#{size:"sm",label:"Small"},
                 #{size:"md",label:"Medium"},#{size:"lg",label:"Large"}] {
        let key = `in-${item.size}-${suffix}`;
        fields.push(input::Input(#{key:key,label:item.label,size:item.size,
            value:kit::val(v,key,item.size),on_change:kit::store(h,key)})
            .with_style(style().width(px(120))));
    }
    kit::sizes(fields)
}

// An embedded search heads a panel: the panel frames it, the hairline separates it from
// the rows, and its text sits on their inset.
fn embedded_search(h, v) {
    let query = kit::val(v, "input.embedded", "");
    let rows = [];
    for name in ["api-gateway", "billing-worker", "edge-03"] {
        if query == "" || name.contains(query) {
            rows.push(row([text(name)]).with_style(style().height(theme_length("metrics.row"))
                .padding_x(theme_length("metrics.inset")).items_center()
                .typography("body").text_color(theme_color("text_primary"))));
        }
    }
    column([input::Input(#{ key: "input.embedded", label: "Filter services", value: query,
        placeholder: "Filter services", appearance: "embedded", on_change: kit::store(h, "input.embedded") })]
        + rows)
        .with_style(style().width(px(280)).gap(theme_spacing("xs")).padding_bottom(theme_spacing("xs"))
            .border(px(1)).border_color(theme_color("border")).radius(theme_radius("lg"))
            .background(theme_color("surface_raised")))
}

fn input_page(h, v) {
    kit::page(h, v, "input", "Input", "Things you type into have a frame. Focus turns the frame ink; errors return on blur.", [
        kit::sec("States", kit::line([
            field(h, v, "input.name", "Name", "Service name"),
            input::Input(#{ key: "input-error", label: "Replicas", value: kit::val(v,"input.replicas","-1"),
                error: true, on_change: kit::store(h,"input.replicas") })
                .with_style(style().width(px(160))),
            input::Input(#{ key: "input-readonly", label: "Region", value: "eu-west", read_only: true })
                .with_style(style().width(px(160))),
            input::Input(#{ key: "input-disabled", label: "Owner", value: "infra", disabled: true })
                .with_style(style().width(px(160))),
        ])),
        kit::sec("Sizes", input_sizes(h, v, "sizes")),
        kit::sec("Density", kit::densities(input_sizes(h, v, "c"), input_sizes(h, v, "k"))),
        kit::sec("Fit to text", kit::line([
            input::Input(#{key:"input-fit",label:"Short reference",value:kit::val(v,"input.fit","Draft"),
                placeholder:"Reference",intrinsic_width:true,on_change:kit::store(h,"input.fit")})
                .with_style(style().min_width(px(0)).max_width(px(220))),
            kit::note("Edit the reference to see the field grow up to its width limit."),
        ])),
        kit::sec("Named part override", kit::pile([
            input::Input(#{key:"input-styled",label:"Styled input",value:kit::val(v,"input.styled","Still a native input"),
                on_change:kit::store(h,"input.styled"),
                style:style().width(px(280)).text_color(theme_color("text_muted")),
                part_styles:#{input:style().text_color(theme_color("text_primary")).radius(theme_radius("sm"))}}),
            kit::note("An explicit input-part style wins over the root fallback. This instance override is not another preset."),
        ])),
        kit::sec("Embedded", embedded_search(h, v)),
    ])
}

fn textarea_page(h, v) {
    kit::page(h, v, "textarea", "Textarea", "Multi-line text keeps the reading line box of body text.", [
        kit::sec("States", kit::line_top([
            textarea::Textarea(#{ key: "textarea.notes", label: "Notes", value: kit::val(v, "textarea.notes", "Line one\n第二行"),
                max_length: 200, show_count: true, on_change: kit::store(h, "textarea.notes") })
                .with_style(style().width(px(280))),
            textarea::Textarea(#{ key: "textarea-error", label: "Reason", value: kit::val(v,"textarea.reason",""), placeholder: "Required",
                error: true, on_change: kit::store(h,"textarea.reason") }).with_style(style().width(px(240))),
        ])),
    ])
}

fn input_group_page(h, v) {
    let control = input::Input(#{ key: "input-group-host", label: "Host", value: kit::val(v, "input-group.host", "example"),
        on_change: kit::store(h, "input-group.host") });
    kit::page(h, v, "input_group", "InputGroup", "One frame around a prefix, an input and a suffix; the frame shows the input's focus.", [
        kit::sec("Prefix and suffix", input_group::InputGroup(#{ label: "URL", prefix: text("https://"), suffix: text(".com"),
            control: control }).with_style(style().width(px(320)))),
    ])
}

fn region_options() {
    [#{ value: "us-east", label: "us-east", group: "Americas" }, #{ value: "us-west", label: "us-west", group: "Americas" },
     #{ value: "eu-west", label: "eu-west", group: "Europe" }, #{ value: "eu-north", label: "eu-north", group: "Europe", disabled: true },
     #{ value: "ap-sg", label: "ap-sg 新加坡", group: "Asia Pacific" }]
}

fn select_page(h, v) {
    let open = kit::val(v, "select.open", false);
    kit::page(h, v, "select", "Select", "A field trigger and a start-aligned list. Arrows move, Enter chooses, typing jumps.", [
        kit::sec("States", kit::line([
            select::Select(#{ key: "select-region", label: "Region", options: region_options(),
                value: kit::val(v, "select.region", "eu-west"), open: open, query: "", width: px(200),
                on_change: kit::store(h, "select.region"), on_open_change: kit::store(h, "select.open") }),
            select::Select(#{ key: "select-empty", label: "Zone", options: region_options(),
                value: kit::val(v, "select.zone", ()), placeholder: "Choose a zone",
                open: kit::val(v, "select.zone_open", false), query: "", width: px(200),
                on_change: kit::store(h, "select.zone"), on_open_change: kit::store(h, "select.zone_open") }),
            select::Select(#{ key: "select-error", label: "Tier",
                options: [#{value:"standard",label:"Standard"},#{value:"dedicated",label:"Dedicated"},#{value:"enterprise",label:"Enterprise"}],
                value: kit::val(v, "select.tier", ()), error: kit::val(v, "select.tier", ()) == (),
                placeholder: "Required", open: kit::val(v, "select.tier_open", false), query: "", width: px(160),
                on_change: kit::store(h, "select.tier"), on_open_change: kit::store(h, "select.tier_open") }),
        ])),
    ])
}

fn combobox_page(h, v) {
    kit::page(h, v, "combobox", "Combobox", "Search within options; multiple selection keeps the check column.", [
        kit::sec("Searchable", combobox::Combobox(#{ key: "combobox-regions", label: "Regions",
            options: region_options(), mode: "multiple", selected: kit::val(v, "combobox.selected", ["us-east"]),
            open: kit::val(v, "combobox.open", false), query: kit::val(v, "combobox.query", ""), searchable: true,
            placeholder: "Regions", width: px(260), on_change: kit::store(h, "combobox.selected"),
            on_open_change: kit::store(h, "combobox.open"), on_query_change: kit::store(h, "combobox.query") })),
    ])
}

fn date_picker_page(h, v) {
    kit::page(h, v, "date_picker", "DatePicker", "A field that opens a month grid. Arrows move by day and week; Enter chooses.", [
        kit::sec("States", kit::line([
            date_picker::DatePicker(#{ key: "date-picker", label: "Release date", value: kit::val(v, "date.value", "2026-10-04"),
                width: px(200), on_change: kit::store(h, "date.value") }),
            date_picker::DatePicker(#{ key: "date-picker-empty", label: "Freeze date",
                value: kit::val(v, "date.freeze", ()), placeholder: "Pick a date",
                width: px(200), on_change: kit::store(h, "date.freeze") }),
        ])),
    ])
}

fn checkbox_page(h, v) {
    let mixed = kit::val(v, "checkbox.mixed", #{checked:false,indeterminate:true});
    kit::page(h, v, "checkbox", "Checkbox", "Click to draw the check or resolve the mixed state. Reduced motion uses the settled mark; focus belongs to the box.", [
        kit::sec("States", kit::line([
            checkbox::Checkbox(#{ checked: kit::val(v, "checkbox.alerts", true), label: "Send alerts",
                on_change: kit::checked(h, "checkbox.alerts") }),
            checkbox::Checkbox(#{ checked: mixed.checked, indeterminate: mixed.indeterminate,
                label: "Some regions", on_change: kit::store(h, "checkbox.mixed") }),
            checkbox::Checkbox(#{ checked: kit::val(v, "checkbox.unchecked", false), label: "Unchecked",
                on_change: kit::checked(h, "checkbox.unchecked") }),
            checkbox::Checkbox(#{ checked: true, label: "Locked", disabled: true }),
        ])),
    ])
}

fn radio_page(h, v) {
    let choice = kit::val(v, "radio.choice", "team");
    kit::page(h, v, "radio", "Radio", "A semantic circle. Use RadioGroup for keyboard selection.", [
        kit::sec("States", kit::line([
            radio::Radio(#{ value: "team", selected: choice == "team", label: "Team", on_select: kit::store(h, "radio.choice") }),
            radio::Radio(#{ value: "org", selected: choice == "org", label: "Organization", on_select: kit::store(h, "radio.choice") }),
            radio::Radio(#{ value: "off", selected: false, label: "Disabled", disabled: true }),
        ])),
    ])
}

fn radio_group_page(h, v) {
    let options = [#{ value: "daily", label: "Daily" }, #{ value: "weekly", label: "Weekly" },
        #{ value: "never", label: "从不", disabled: true }];
    kit::page(h, v, "radio_group", "RadioGroup", "One tab stop; arrows change the selection and the ring shows focus.", [
        kit::sec("Vertical", radio_group::RadioGroup(#{ value: kit::val(v, "radio_group.cadence", "daily"), label: "Cadence",
            options: options, on_change: kit::store(h, "radio_group.cadence") })),
        kit::sec("Horizontal", radio_group::RadioGroup(#{ value: kit::val(v, "radio_group.cadence", "daily"), label: "Cadence inline",
            orientation: "horizontal", options: options, on_change: kit::store(h, "radio_group.cadence") })),
    ])
}

fn switch_page(h, v) {
    kit::page(h, v, "switch", "Switch", "A capsule track with a round thumb, for settings that apply at once.", [
        kit::sec("Settings rows",column([
            switch_control::Switch(#{key:"switch-background",label:"Background sync",description:"Keep this device up to date while the app is open.",
                placement:"end",checked:kit::val(v,"switch.background",true),on_change:kit::store(h,"switch.background")}),
            switch_control::Switch(#{key:"switch-small",label:"Quiet hours",description:"Pause non-urgent notifications · 暂停非紧急通知",
                size:"sm",placement:"end",checked:kit::val(v,"switch.quiet",false),on_change:kit::store(h,"switch.quiet")}),
        ]).with_style(style().width(px(480)).gap(theme_spacing("md")))),
        kit::sec("States", kit::line([
            switch_control::Switch(#{ checked: kit::val(v, "switch.updates", true), label: "Automatic updates",
                on_change: kit::store(h, "switch.updates") }),
            switch_control::Switch(#{ checked: false, label: "Telemetry", on_change: h.noop }),
            switch_control::Switch(#{ checked: true, label: "Saving", loading: true }),
            switch_control::Switch(#{ checked: false, label: "Disabled", disabled: true }),
        ])),
    ])
}

fn slider_page(h, v) {
    kit::page(h, v, "slider", "Slider", "A fader cap on a thin track; arrows step the value.", [
        kit::sec("Value", slider::Slider(#{ key: "slider-volume", value: kit::val(v, "slider.volume", 40.0), min: 0.0,
            max: 100.0, step: 5.0, label: "Volume", on_change: kit::store(h, "slider.volume") })),
    ])
}

fn range_slider_page(h, v) {
    kit::page(h, v, "range_slider", "RangeSlider", "Two caps on one track, each its own tab stop.", [
        kit::sec("Range", range_slider::RangeSlider(#{ key: "range-price", label: "Price",
            values: kit::val(v, "range.price", #{ low: 20.0, high: 70.0 }), min: 0.0, max: 100.0, step: 1.0,
            minimum_gap: 5.0, orientation: "horizontal", disabled: false, low_label: "Minimum", high_label: "Maximum",
            show_value: true, on_change: kit::store(h, "range.price") })),
    ])
}

fn label_page(h, v) {
    kit::page(h, v, "label", "Label", "Field labels are body text at 400; descriptions are captions.", [
        kit::sec("Variants", kit::pile([
            label::Label(#{ text: "Service name", required: true }),
            label::Label(#{ text: "Owner team", description: "Gets paged for incidents." }),
        ])),
    ])
}

fn form_field_page(h, v) {
    kit::page(h, v, "form_field", "FormField", "A label, a control and its description or error, linked for assistive technology.", [
        kit::sec("States", kit::line([
            form_field::FormField(#{ id: "ff-name", label: "Service name", required: true, description: "Lowercase and dashes.",
                control: field(h, v, "form-field.name", "Service name", "api-gateway") }),
            form_field::FormField(#{ id: "ff-replicas", label: "副本数", error: "必须是正整数",
                control: input::Input(#{ key: "ff-replicas-input", label: "副本数", value: "-1", error: true,
                    on_change: h.noop }).with_style(style().width(px(160))) }),
        ])),
    ])
}
