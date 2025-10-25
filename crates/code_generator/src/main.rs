use code_generator::{
    Context, InLocation, PathItem, PathName, SwaggerFile, ToRustTypeName, Type, TypePath,
    TypeTagged, TypeUntagged,
};

pub fn main() -> anyhow::Result<()> {
    let mut deserializer = serde_json::Deserializer::from_reader(std::io::stdin());

    let result: SwaggerFile = match serde_path_to_error::deserialize(&mut deserializer) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error at path: {}", e.path());
            return Err(e.into());
        }
    };
    let names = result
        .definitions
        .keys()
        .map(|name| {
            (
                TypePath(format!("#/definitions/{}", name)),
                name.clone().replace(r#"V3."#, ""),
            )
        })
        .collect::<std::collections::HashMap<TypePath, String>>();
    //    println!(" Definitions : {:?}", names.keys());
    let mut context = Context {
        strip_prefix: None,
        types: names,
        extra_name: std::cell::RefCell::new(None),
    };
    let mut scope = codegen::Scope::new();

    for (name, ty) in &result.definitions {
        match ty.schema_object {
            TypeUntagged::Tagged(ref tagged) => match tagged {
                TypeTagged::Object {
                    properties,
                    required,
                    additional_properties,
                } => {
                    let mut fields = Vec::new();

                    if let Some(props) = &properties {
                        for (prop_name, prop_type) in props {
                            context.extra_name.replace(Some(prop_name.clone()));
                            let rust_type = prop_type
                                .schema_object
                                .to_rust_type_name(&mut context, &mut scope)?;
                            if let Some(req) = &required {
                                if req.contains(prop_name) {
                                    fields.push(codegen::Field::new(prop_name, rust_type));
                                }
                            } else {
                                fields.push(codegen::Field::new(
                                    prop_name,
                                    format!("Option<{}>", rust_type),
                                ));
                            }
                        }
                    }
                    if let Some(add_props) = &additional_properties {
                        let typea = add_props
                            .schema_object
                            .to_rust_type_name(&mut context, &mut scope)?;
                        fields.push(codegen::Field::new("additional_properties", typea));
                    }
                    let struc = scope
                        .new_struct(&name.replace(r#"V3."#, ""))
                        .derive("Debug")
                        .derive("Serialize")
                        .derive("Deserialize");
                    for field in fields {
                        struc.push_field(field);
                    }
                }
                _ => {
                    println!("Type: {} is Tagged but not an Object", name);
                }
            },
            _ => {
                println!("Type: {} is Untagged", name);
            }
        }
    }

    for (path, item) in result.paths {
        if let Some(op) = item.get {
            if let Some(params) = op.parameters {
                let struct_vars = params
                    .iter()
                    .filter(|x| x.in_ == InLocation::Path)
                    .collect::<Vec<_>>();
            }
        }
    }

    println!("{}", scope.to_string());

    Ok(())
}
