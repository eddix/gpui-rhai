// Charts use the same presets and live appearance switching as every other
// Gallery control. The tiny synthetic map is a Host-owned demonstration fixture.
import "gallery/kit" as kit;
import "charts/chart" as chart;
import "charts/bar_chart" as bar;
import "charts/line_chart" as line;
import "charts/pie_chart" as pie;
import "charts/map_chart" as map;
import "components/button" as button;

fn pages() {
    [
        #{id:"charts.overview",title:"Chart",group:"Charts"},
        #{id:"charts.bar",title:"BarChart",group:"Charts"},
        #{id:"charts.line",title:"LineChart",group:"Charts"},
        #{id:"charts.pie",title:"PieChart",group:"Charts"},
        #{id:"charts.map",title:"MapChart",group:"Charts"},
    ]
}

fn rows() {
    [#{id:"jan",month:"Jan",value:18.0,other:12.0},
     #{id:"feb",month:"Feb",value:31.0,other:22.0},
     #{id:"mar",month:"Mar",value:26.0,other:35.0},
     #{id:"apr",month:"Apr",value:44.0,other:29.0},
     #{id:"may",month:"May",value:38.0,other:41.0}]
}

fn selected(v) { kit::val(v,"charts.selection",[]) }
fn frame(node) {
    node.with_style(style().width(relative(1.0)).height(px(300)).min_width(px(0)))
}

fn demo(id, h, v) {
    let props = #{key:id,data:rows(),key_dimension:"id",title:"Monthly activity",
        encode:#{x:"month",y:"value"},selected_keys:selected(v),on_select:h.chart_select};
    if id == "charts.bar" { return bar::BarChart(props); }
    if id == "charts.line" { return line::LineChart(props); }
    if id == "charts.pie" {
        props.encode = #{name:"month",value:"value"};
        return pie::PieChart(props);
    }
    if id == "charts.map" {
        props.data = [#{id:"north",name:"north",value:72.0},#{id:"south",name:"south",value:34.0}];
        props.encode = #{name:"name",value:"value"};
        props.map = "demo_map";
        props.title = "Host-owned map data";
        return map::MapChart(props);
    }
    chart::Chart(#{key:"gallery-chart",data:rows(),key_dimension:"id",selected_keys:selected(v),
        on_select:h.chart_select,on_brush_change:h.chart_brush,
        spec:#{title:"Shared coordinates and selection",brush:"x",
            series:[#{key:"actual",kind:"bar",name:"Actual",encode:#{x:"month",y:"value"}},
                    #{key:"target",kind:"line",name:"Target",encode:#{x:"month",y:"other"}}]}})
}

fn view(ctx, h, v, id) {
    if !(id in ["charts.overview","charts.bar","charts.line","charts.pie","charts.map"]) { return (); }
    kit::page(h,v,id,"Charts","Click a datum, or use the chart's keyboard navigation. Selection is controlled by this application.",[
        kit::sec("Interactive chart",frame(demo(id,h,v))),
        kit::sec("Selection",kit::line([
            text(`Selected keys: ${selected(v)}`).accessibility_role("status"),
            button::Button(#{text:"Clear selection",variant:"secondary",on_click:kit::put(h,"charts.selection",[])}),
        ])),
    ])
}
