use clap::{Arg, Command};

use code_indexing::CodeIndex;
use env_logger;
use log;
use serde::Deserialize;
use tide::prelude::*;
use tide::Request;

#[derive(Debug, Deserialize)]
struct ParseFileReq {
    file: String,
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
    app.listen(addr).await?;
    Ok(())
}

async fn api_parse_file(mut req: Request<()>) -> tide::Result {
    let ParseFileReq { file } = req.body_json().await?;
    let mut indexing = CodeIndex::new();
    match indexing.parse_file(&file) {
        Ok(_) => Ok(json!({
            "code": 200,
            "message": "success",
            "data": indexing,
        })
        .into()),
        Err(e) => Ok(json!({
            "code": 5001,
            "message": format!("{} Failed to parse file", e.to_string())
        })
        .into()),
    }
}

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
        <div id="f20333b98be84c3497bdb4b930129314" class="chart-container" style="width:1200px; height:1000px; "></div>
        <script>
            var chart = echarts.init(
                document.getElementById('f20333b98be84c3497bdb4b930129314'), 'white', {{ renderer: 'canvas' }});
            var option = {{
                tooltip: {{
                    trigger: 'item',
                    triggerOn: 'mousemove'
                }},
            series: [
                {{
                    type: 'tree',
                    data: ${data}$,
                    top: '1%',
                    left: '7%',
                    bottom: '1%',
                    right: '20%',
                    symbolSize: 7,
                    label: {{
                        position: 'left',
                        verticalAlign: 'middle',
                        align: 'right',
                        fontSize: 9
                     }},
                leaves: {{
                    label: {{
                        position: 'right',
                        verticalAlign: 'middle',
                        align: 'left'
                    }}
                }},
                emphasis: {{
                    focus: 'descendant'
                }},
                expandAndCollapse: true,
                animationDuration: 550,
                animationDurationUpdate: 750
            }}
            ]
            }};
            chart.setOption(option);
        </script>
    </body>
    
    </html>

    "#;
    template.to_string()
}
