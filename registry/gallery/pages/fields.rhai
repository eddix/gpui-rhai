// Gallery spec pages: fields and choices.

import "gallery/kit" as kit;
import "components/input" as input;
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

fn field(h, v, key, label_text, placeholder) {
    input::Input(#{ key: key, label: label_text, value: kit::val(v, key, ""), placeholder: placeholder,
        on_change: kit::store(h, key) }).with_style(style().width(px(200)))
}

fn input_sizes(h, v, suffix) {
    kit::sizes([
        input::Input(#{ key: `in-xs-${suffix}`, label: "Extra small", value: "xs", size: "xs", on_change: h.noop })
            .with_style(style().width(px(120))),
        input::Input(#{ key: `in-sm-${suffix}`, label: "Small", value: "sm", size: "sm", on_change: h.noop })
            .with_style(style().width(px(120))),
        input::Input(#{ key: `in-md-${suffix}`, label: "Medium", value: "md", on_change: h.noop })
            .with_style(style().width(px(120))),
        input::Input(#{ key: `in-lg-${suffix}`, label: "Large", value: "lg", size: "lg", on_change: h.noop })
            .with_style(style().width(px(120))),
    ])
}

fn input_page(h, v) {
    kit::page(h, v, "input", "Input", "Things you type into have a frame. Focus turns the frame ink; errors return on blur.", [
        kit::sec("States", kit::line([
            field(h, v, "input.name", "Name", "Service name"),
            input::Input(#{ key: "input-error", label: "Replicas", value: "-1", error: true, on_change: h.noop })
                .with_style(style().width(px(160))),
            input::Input(#{ key: "input-readonly", label: "Region", value: "eu-west", read_only: true })
                .with_style(style().width(px(160))),
            input::Input(#{ key: "input-disabled", label: "Owner", value: "infra", disabled: true })
                .with_style(style().width(px(160))),
        ])),
        kit::sec("Sizes", input_sizes(h, v, "sizes")),
        kit::sec("Density", kit::densities(input_sizes(h, v, "c"), input_sizes(h, v, "k"))),
    ])
}

fn textarea_page(h, v) {
    kit::page(h, v, "textarea", "Textarea", "Multi-line text keeps the reading line box of body text.", [
        kit::sec("States", kit::line_top([
            textarea::Textarea(#{ key: "textarea.notes", label: "Notes", value: kit::val(v, "textarea.notes", "Line one\n第二行"),
                max_length: 200, show_count: true, on_change: kit::store(h, "textarea.notes") })
                .with_style(style().width(px(280))),
            textarea::Textarea(#{ key: "textarea-error", label: "Reason", value: "", placeholder: "Required",
                error: true, on_change: h.noop }).with_style(style().width(px(240))),
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
            select::Select(#{ key: "select-empty", label: "Zone", options: region_options(), value: (),
                placeholder: "Choose a zone", open: false, query: "", width: px(200), on_change: h.noop }),
            select::Select(#{ key: "select-error", label: "Tier", options: region_options(), value: (), error: true,
                placeholder: "Required", open: false, query: "", width: px(160), on_change: h.noop }),
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
            date_picker::DatePicker(#{ key: "date-picker-empty", label: "Freeze date", value: (), placeholder: "Pick a date",
                width: px(200), on_change: h.noop }),
        ])),
    ])
}

fn checkbox_page(h, v) {
    kit::page(h, v, "checkbox", "Checkbox", "The box shows focus; the row is the target, so the box stays on the content edge.", [
        kit::sec("States", kit::line([
            checkbox::Checkbox(#{ checked: kit::val(v, "checkbox.alerts", true), label: "Send alerts",
                on_change: kit::checked(h, "checkbox.alerts") }),
            checkbox::Checkbox(#{ checked: false, indeterminate: true, label: "Some regions", on_change: h.noop }),
            checkbox::Checkbox(#{ checked: false, label: "Unchecked", on_change: h.noop }),
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
    kit::page(h, v, "switch", "Switch", "A rectangular track with a square thumb, for settings that apply at once.", [
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
