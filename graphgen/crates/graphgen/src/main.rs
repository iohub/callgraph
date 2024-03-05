use std::sync::Mutex;

use clap::{Arg, Command};
use code_indexing::graph::GraphNode;
use code_indexing::CodeIndex;
use env_logger;
use http_types::headers::HeaderValue;
use lazy_static::lazy_static;
use log::{error, info};
use serde::Deserialize;

use tide::prelude::*;
use tide::security::{CorsMiddleware, Origin};
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

#[derive(Debug, Deserialize)]
struct CallGraphHtmlReq {
    depth: i32,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    env_logger::init();

    let args = Command::new("graphgen")
        .arg(Arg::new("listen-addr").long("listen-addr"))
        .arg(Arg::new("project-dir").long("project-dir"))
        .get_matches();

    let addr = args.get_one::<String>("listen-addr").unwrap();
    let project_dir = args.get_one::<String>("project-dir").unwrap();

    if let Err(e) = CONTEXT
        .lock()
        .unwrap()
        .code_index
        .parse_project(project_dir)
    {
        error!("parse_project error {}", e);
    }

    let mut app = tide::new();
    let cors = CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false);
    app.with(cors);

    app.at("/codeindex/parse/file").post(api_parse_file);
    app.at("/codeindex/load").post(api_load_codeindex);
    app.at("/callgraph/json").post(api_callgraph_json);
    app.at("/codeindex/functions").get(api_function_list);
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

async fn api_function_list(req: Request<()>) -> tide::Result {
    let result = CONTEXT.lock().unwrap().code_index.function_list();

    Ok(json!({
        "code": 200,
        "message": "success",
        "data": result,
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
    let CallGraphHtmlReq { depth } = req.query()?;
    let host = req.local_addr().unwrap();
    let html_content = echart_tree_template()
        .replace("${host}$", host)
        .replace("${depth}$", &depth.to_string());
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
        <h2>Choose a function</h2>
        <input type="text" id="keywordInput" placeholder="Type to filter...">
        <select id="dynamicSelect" name="dynamicSelect" class="styled-select">
            <option value="">Select an option...</option>
        </select>
        <div id="f20333b98be84c3497bdb4b930129314" class="chart-container" style="width: 80vw; height: 1000px; "></div>
        <script>
            var chart = echarts.init(
                document.getElementById('f20333b98be84c3497bdb4b930129314'), 'white', { renderer: 'canvas' });
    
            document.getElementById('dynamicSelect').addEventListener('change', function() {
                draw_function_graph(this.value, ${depth}$); 
            });
    
            function draw_function_graph(func, depth) {
                const url = 'http://${host}$/callgraph/json';
                const postData = {
                    "function": func,
                    "depth": depth
                };
                fetch(url, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    }, 
                    body: JSON.stringify(postData),
                })
                .then(response => response.json())
                .then(resp => { 
    
                    var option = {
                        tooltip: {
                            trigger: 'item',
                            triggerOn: 'mousemove'
                        },
                        series: [
                            {
                                type: 'tree',
                                data: [ resp.data ], 
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
                                        return  params.data.name + ' (' + params.data.value + ')' ; 
                                        // return '{b|' + params.data.name + ' (' + params.data.value + ')' + '}';
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
                })
                .catch((error) => {
                    console.error('Error:', error);
                });
            }
     
            document.addEventListener('DOMContentLoaded', function() { 
                const url = 'http://${host}$/codeindex/functions';
                fetch(url, {
                    method: 'GET',
                    headers: {
                        'Content-Type': 'application/json',
                    }, 
                })
                .then(response => response.json())
                .then(resp => { 
                    const selectElement = document.getElementById('dynamicSelect');
                    selectElement.innerHTML = '';
                    resp.data.forEach(item => {
                        const option = document.createElement('option');
                        option.value = item;
                        option.text = item;
                        selectElement.appendChild(option);
                    });
                })
                .catch((error) => {
                    console.error('Error:', error);
                });
                const keywordInput = document.getElementById('keywordInput');
                const optionsSelect = document.getElementById('dynamicSelect');
                keywordInput.addEventListener('input', function() {
                    const keyword = keywordInput.value.toLowerCase();
                    const options = optionsSelect.options;
                    for (let i = 0; i < options.length; i++) {
                        const option = options[i];
                        const optionText = option.text.toLowerCase();
                        if (optionText.includes(keyword)) {
                            option.style.display = '';
                        } else {
                            option.style.display = 'none';
                        }
                    }
                });
            });
    
        </script>
    </body> 
    
    </html>

    "#;
    template.to_string()
}
