// Gallery acceptance pages for controlled hierarchical browsing and native ranking.
import "gallery/kit" as kit;
import "components/column_browser" as browser;
import "components/table" as table;
import "components/button" as button;

fn pages() { [
    #{ id: "column_browser", title: "ColumnBrowser", group: "Lists" },
    #{ id: "formatted_table", title: "Formatted Table", group: "Lists" },
] }
fn view(ctx, h, v, id) {
    if id == "column_browser" { browser_page(h, v) }
    else if id == "formatted_table" { formatted_page(ctx, h, v) }
    else { () }
}

fn browser_page(h, v) {
    let event = kit::val(v, "browser.selection", #{ path: [], column: "", key: "" });
    let path = event.path;
    let columns = [#{ key: "tenant", title: "Tenant", items: [
        #{ key: "payments", title: "Payments", secondary: "Production", has_children: true },
        #{ key: "catalog", title: "商品目录", secondary: "Production", has_children: true },
        #{ key: "archived", title: "Archived", disabled: true, has_children: true },
    ] }];
    if path.len > 0 {
        columns.push(#{ key: `directory-${path[0]}`, title: "Directory", items: [
            #{ key: "checkout", title: "Checkout", meta: "3 keys", has_children: true },
            #{ key: "empty", title: "Empty directory", meta: "0 keys", has_children: true },
            #{ key: "retry", title: "Failed directory", has_children: true },
        ] });
    }
    let failed = kit::val(v, "browser.failed", true);
    if path.len > 1 {
        columns.push(#{ key: `keys-${path[0]}-${path[1]}`, title: "Key", items: if path[1] == "empty" { [] } else { [
            #{ key: "timeout_ms", title: "timeout_ms", secondary: "Request timeout", meta: "5000" },
            #{ key: "retry_count", title: "retry_count", secondary: "Maximum retries", meta: "3" },
            #{ key: "非常长的配置键", title: "非常长的配置键 — long titles retain their full accessible name", meta: "true" },
        ] }, empty_text: "No keys in this directory",
            error: if path[1] == "retry" && failed { "Could not load keys. Retry this column." } else { () } });
    }
    let opened = kit::val(v, "browser.activation", #{ key: "None" });
    let detail = if path.len < 3 { () } else { column([
        text(path[2]).with_style(style().typography("subtitle")),
        text("Caller-owned detail. Enter or double-click activates this leaf.").selectable(true),
    ]).with_style(style().padding(theme_length("metrics.inset")).gap(theme_spacing("sm"))) };
    kit::page(h, v, "column_browser", "ColumnBrowser", "Controlled virtual columns. Arrows follow logical depth; ancestors stay selected and each column owns its loading result.", [
        kit::sec("Configuration", browser::ColumnBrowser(#{ key: "config-browser", label: "Configuration", columns: columns,
            path: path, height: 300.0, column_width: 240.0, detail: detail,
            on_select: kit::store(h, "browser.selection"), on_activate: kit::store(h, "browser.activation"), on_retry: kit::put(h, "browser.failed", false) })),
        kit::sec("Accepted events", kit::pile([kit::note(`Selected path: ${path}`), kit::note(`Activated leaf: ${opened.key}`)])),
        kit::sec("Loading", browser::ColumnBrowser(#{ key: "loading-browser", label: "Loading example", height: 160.0,
            columns: [#{ key: "root", title: "Tenant", items: [#{ key:"payments",title:"Payments",has_children:true }], selected:"payments" },
                #{ key: "child", title: "Directory", items: [], loading: true }] })),
    ])
}
fn formatted_page(ctx, h, v) {
    let sort = kit::val(v, "formatted.sort", ());
    kit::page(h, v, "formatted_table", "Formatted Table", "30,000 Rust-owned rows. Display text stays formatted; sortable headers compare independent numeric ranks, with missing values last in both directions.", [
        kit::sec("Native ranking", table::Table(#{ key: "formatted-native", label: "Formatted native rows", row_key: "id",
            rows: ctx.get_native_collection("gallery_formatted_rows"), height: 320.0, sort: sort,
            columns: [
                #{ key:"name",title:"Name",sortable:true,width:#{kind:"flex",value:1} },
                #{ key:"status",title:"Status",sortable:true,sort_key:"status_rank",width:#{kind:"fixed",value:160} },
                #{ key:"duration",title:"Duration",sortable:true,sort_key:"duration_seconds",width:#{kind:"fixed",value:120} },
            ], on_sort_change: kit::store(h, "formatted.sort") })),
        kit::sec("Accepted sort", kit::note(`Controlled column sort: ${sort}`)),
        kit::sec("Contract", kit::note("sort and sort_change name the displayed column, not its rank field. Search, grouping and resizing continue to use the column key. Click a header three times for ascending, descending, then source order.")),
    ])
}
