use std::{env, error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = env::args().skip(1).collect();
    let source = arguments
        .first()
        .ok_or("Expected the glslang source directory")?;
    let changes = fs::read_to_string(Path::new(source).join("CHANGES.md"))?;
    let version = changes
        .lines()
        .find_map(|line| {
            line.strip_prefix("## ")
                .and_then(|heading| heading.split_whitespace().next())
        })
        .ok_or("glslang CHANGES.md has no version heading")?;
    let (numeric, flavor) = version.split_once('-').unwrap_or((version, ""));
    let components: Vec<_> = numeric.split('.').collect();
    if components.len() != 3 {
        return Err("Expected a three-component glslang version".into());
    }
    for component in &components {
        component.parse::<u32>()?;
    }
    let render = |template: &str| {
        template
            .replace("<major>", components[0])
            .replace("<minor>", components[1])
            .replace("<patch>", components[2])
            .replace(
                "<-flavor>",
                &if flavor.is_empty() {
                    String::new()
                } else {
                    format!("-{flavor}")
                },
            )
            .replace("<flavor>", flavor)
    };
    match arguments.as_slice() {
        [_, pattern] => print!("{}", render(pattern)),
        [_, input_flag, template, output_flag, output]
            if input_flag == "-i" && output_flag == "-o" =>
        {
            let header = render(&fs::read_to_string(template)?);
            let output = Path::new(output);
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output, header)?;
        }
        _ => {
            return Err(
                "Usage: glslang-info <source> <pattern> | <source> -i <template> -o <header>"
                    .into(),
            );
        }
    }
    Ok(())
}
