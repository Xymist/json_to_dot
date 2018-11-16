#[macro_use]
extern crate error_chain;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod errors {
    error_chain!{}
}

use crate::errors::*;

#[derive(Serialize, Deserialize)]
struct Node {
    short_code: String,
    text: String,
    description: String,
    doneness: i32
}

impl Node {
    fn colour(&self) -> String {
        match self.doneness {
            100 => String::from("10"),
            0...99 => format!("{}", (((self.doneness as f32) + 1.0)/10.0).ceil() as i32),
            _ => panic!("Doneness was not a number!")
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Dependency {
    node: String,
    depends_on: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Graph {
    nodes: Vec<Node>,
    dependencies: Vec<Dependency>,
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }

}

fn run() -> Result<()> {
    use std::fs;

    let graph = take_file_input("./data/test_graph.json")
        .chain_err(|| "unable to get deserialized JSON")?;
    let node_list = assemble_nodes(graph.nodes)
        .chain_err(|| "Could not assemble Node definitions")?;
    let dep_list = assemble_dependencies(&graph.dependencies)
        .chain_err(|| "could not assemble dependency definitions")?;

    let temp_file_name = write_temp_gv_file(&node_list, &dep_list)
        .chain_err(|| "unable to write temp file, aborting.")?;

    let output_file_name = write_output_image(&temp_file_name)
        .chain_err(|| "unable to write output image, aborting.")?;

    fs::remove_file(temp_file_name)
        .chain_err(|| "unable to remove temporary GV file; do you have write permission?")?;

    println!("Generated graph: {}", output_file_name);

    Ok(())
}

fn take_file_input(filename: &str) -> Result<Graph> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut input = String::new();

    let mut file = File::open(filename)
        .chain_err(|| "unable to open JSON file")?;

    file.read_to_string(&mut input)
        .chain_err(|| "unable to read JSON file")?;

    let graph: Graph = serde_json::from_str(&input)
        .chain_err(|| "unable to deserialize JSON")?;

    Ok(graph)
}

fn assemble_nodes(nodes: Vec<Node>) -> Result<String> {
    let mut node_list = String::new();

    for n in nodes {
        node_list.push_str(
            format!(
                "\t\"{}\" [label=\"{}\\n{}\",color=\"{}\"]\n",
                n.short_code,
                n.text,
                n.description,
                n.colour()
            ).as_ref()
        )
    }

    Ok(node_list)

}

fn assemble_dependencies(dependencies: &[Dependency]) -> Result<String>{
    let dep_list: Vec<String> = dependencies.iter().map(dependency_map).collect();
    Ok(dep_list.concat())
}

fn dependency_map(dependency: &Dependency) -> String {
    let res: Vec<String> = dependency
        .depends_on
        .iter()
        .map(|dp| format!(
                "\t\"{}\" -> \"{}\"\n",
                dependency.node,
                dp
            ).to_owned()
        )
        .collect();

    res.concat()
}

fn write_temp_gv_file(node_list: &str, dep_list: &str) -> Result<String> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut temp_file = File::create("temp.gv")
        .chain_err(|| "could not create temporary GV file; do you have write permission?")?;

    temp_file.write_all(
        format!(
            "digraph G {{\nnode [colorscheme=rdylgn10]\n{}\n\n{}\n}}",
            node_list,
            dep_list
        ).as_bytes()
    ).chain_err(|| "unable to write Graph definition to file")?;

    Ok("temp.gv".to_owned())
}

fn write_output_image(temp_file_name: &str) -> Result<String> {
    use std::fs::File;
    use std::io::prelude::*;
    use std::process::Command;

    let mut output_file = File::create("output.png")
        .chain_err(|| "could not create output PNG; do you have write permission?")?;

    let output = Command::new("dot")
        .arg("-Tpng")
        .arg(temp_file_name)
        .output()
        .chain_err(|| "Dot failed to render graph")?;

    output_file.write_all(&output.stdout)
        .chain_err(|| "write to PNG failed, aborting.")?;

    Ok("output.png".to_owned())
}
