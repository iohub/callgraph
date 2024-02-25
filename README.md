# callgraph

## Usage

1. run graphgen on your project source directory.

```shell
graphgen --listen-addr 127.0.0.1:12800
```

2. open `http://127.0.0.1:12800/callgraph/html?depth=4` 
3. choose a function to draw callgraph.

- screenshot:

<img src="img/graph_demo2.png" alt="demo2"/>

- usage gif:
<img src="img/callgraph_demo1.gif" alt="demo1"/>

## Todo

- [x] supports `typescript` (completed)
- [ ] supports `Java`、`Golang`
