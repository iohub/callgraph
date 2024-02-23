import networkx as nx
from pyecharts import options as opts
from pyecharts.charts import Graph

# Create a directed graph
G = nx.DiGraph()

# Add edges to create the tree structure
G.add_edge("A", "B")
G.add_edge("A", "C")
G.add_edge("B", "D")
G.add_edge("B", "E")
G.add_edge("C", "F")
G.add_edge("C", "G")

# Prepare the data for Pyecharts
nodes = [{"name": node, "symbolSize":  50} for node in G.nodes]
links = [{"source": edge[0], "target": edge[1]} for edge in G.edges]

# Create the Pyecharts Graph chart
graph = (
    Graph(init_opts=opts.InitOpts(width="1000px", height="600px"))
    .add(
        "",
        nodes,
        links,
        repulsion=5000,
        linestyle_opts=opts.LineStyleOpts(curve=0.2),
        label_opts=opts.LabelOpts(is_show=True),
    )
    .set_global_opts(
        title_opts=opts.TitleOpts(title="Tree Diagram Example"),
        legend_opts=opts.LegendOpts(is_show=False),
    )
)

# Render the chart
graph.render("tree_diagram.html")