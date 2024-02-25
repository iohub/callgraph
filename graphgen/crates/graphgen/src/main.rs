use std::sync::{Mutex, Once};

use clap::{Arg, Command};

use code_indexing::CodeIndex;
use env_logger;
use lazy_static::lazy_static;
use log;
use serde::Deserialize;
use serde_json;
use tide::prelude::*;
use tide::{Request, Response, StatusCode};

struct GlobalSingleton {
    code_index: CodeIndex,
}

lazy_static! {
    static ref CONTEXT: Mutex<GlobalSingleton> = Mutex::new(GlobalSingleton {
        code_index: CodeIndex::new(),
    });
}

#[derive(Debug, Deserialize)]
struct ParseFileReq {
    file: String,
    load: bool,
}

#[derive(Debug, Deserialize)]
struct LoadCodeIndexReq {
    file: String,
}

#[derive(Debug, Deserialize)]
struct CallGraphRenderReq {
    function: String,
    depth: i32,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    env_logger::init();

    let args = Command::new("graphgen")
        .arg(Arg::new("listen-addr").long("listen-addr"))
        .get_matches();

    let addr = args.get_one::<String>("listen-addr").unwrap();

    let mut app = tide::new();
    app.at("/codeindex/parse/file").post(api_parse_file);
    app.at("/codeindex/load").post(api_load_codeindex);
    app.at("/callgraph/json").post(api_callgraph_json);
    app.at("/callgraph/html").get(api_callgraph_html);
    app.listen(addr).await?;
    Ok(())
}

async fn api_parse_file(mut req: Request<()>) -> tide::Result {
    let ParseFileReq { file, load } = req.body_json().await?;
    let mut indexing = CodeIndex::new();
    match indexing.parse_file(&file) {
        Ok(_) => {
            if load {
                CONTEXT.lock().unwrap().code_index = indexing.clone();
            }
            Ok(json!({
                "code": 200,
                "message": "success",
                "data": indexing,
            })
            .into())
        }
        Err(e) => Ok(json!({
            "code": 5001,
            "message": format!("{} Failed to parse file", e.to_string())
        })
        .into()),
    }
}

async fn api_load_codeindex(mut req: Request<()>) -> tide::Result {
    let LoadCodeIndexReq { file } = req.body_json().await?;
    Ok(json!({
        "code": 200,
        "message": "success",
    })
    .into())
}

async fn api_callgraph_json(mut req: Request<()>) -> tide::Result {
    let CallGraphRenderReq { function, depth } = req.body_json().await?;
    let result = CONTEXT
        .lock()
        .unwrap()
        .code_index
        .serde_tree(&function, depth);
    match result {
        None => Ok(json!({
            "code": 300,
            "message": "no graph generated"
        })
        .into()),
        Some(graph) => Ok(json!({
            "code": 200,
            "message": "success",
            "data": graph,
        })
        .into()),
    }
}

async fn api_callgraph_html(req: Request<()>) -> tide::Result {
    let CallGraphRenderReq { function, depth } = req.query()?;
    let result = CONTEXT
        .lock()
        .unwrap()
        .code_index
        .serde_tree(&function, depth);

    let data = match result {
        Some(graph) => serde_json::to_string(&graph).unwrap_or("{}".to_string()),
        None => "{}".to_string(),
    };

    let html_content = echart_tree_template().replace("${data}$", &data);
    let mut res = Response::new(StatusCode::Ok);
    res.set_body(html_content);
    res.set_content_type(tide::http::mime::HTML);
    Ok(res)
}

///
/// html templates
///
fn echart_tree_template() -> String {
    let template = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <title>CallGraph</title>
        <script type="text/javascript" src="https://assets.pyecharts.org/assets/v5/echarts.min.js"></script>
    </head>
    
    <body>
        <div id="f20333b98be84c3497bdb4b930129314" class="chart-container" style="width: 80vw; height:1000px; "></div>
        <script>
            var chart = echarts.init(
                document.getElementById('f20333b98be84c3497bdb4b930129314'), 'white', { renderer: 'canvas' });
            var option = {
                tooltip: {
                    trigger: 'item',
                    triggerOn: 'mousemove'
                },
            series: [
                {
                    type: 'tree',
                    data: [${data}$],
                    top: '1%',
                    left: '7%',
                    bottom: '1%',
                    right: '20%',
                    symbolSize: 7,
                    label: {
                        position: 'inside',
                        verticalAlign: 'middle',
                        align: 'center',
                        formatter: function(params) { 
                            return '{b|' + params.data.name + ' (' + params.data.value + ')' + '}';
                        },
                        fontSize: 16,
                        rich: { 
                            b: {
                                backgroundColor: '#eee',
                                width:  320,
                                height:  32,
                                align: 'center',
                                borderRadius:  4,
                                fontSize: 16,
                                color: '#333'
                            }
                        }
                    }, 
                    leaves: {
                        label: {
                            position: 'right',
                            verticalAlign: 'middle',
                            align: 'left'
                        }
                    },
                    emphasis: {
                        focus: 'descendant'
                    },
                    expandAndCollapse: true,
                    animationDuration: 550,
                    animationDurationUpdate: 750
                }
            ]
            };
            chart.setOption(option);
        </script>
    </body>
    
    </html>

    "#;
    template.to_string()
}
