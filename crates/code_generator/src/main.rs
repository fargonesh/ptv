use std::rc::Rc;

use code_generator::{
    Context, InLocation, PathItem, PathName, SwaggerFile, ToRustTypeName, Type, TypePath,
    TypeTagged, TypeUntagged,
};
use heck::*;
use itertools::Itertools;

// TODO: make this a proc macro for a struct of type and hold async http client methods
// // #[derive(SwaggerApi)]
// // #[swagger_api(path = "/path/to/api.json", strip_prefix = "V3.")]
// // struct MyApi;

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
                // TODO: make names properly configured, not just strip V3.
                name.clone().replace(r#"V3."#, ""),
            )
        })
        .collect::<std::collections::HashMap<TypePath, String>>();
    //    println!(" Definitions : {:?}", names.keys());
    //
    let context = Rc::new(Context {
        scope: std::cell::RefCell::new(codegen::Scope::new()),
        extra_types: std::collections::HashMap::new().into(),
        strip_prefix: Default::default(),
        types: names,
        extra_name: Default::default(),
    });

    for (name, ty) in &result.definitions {
        let context = context.clone();
        match ty.schema_object {
            TypeUntagged::Tagged(ref tagged) => match tagged {
                TypeTagged::Object {
                    properties,
                    required,
                    additional_properties,
                } => {
                    let mut fields = Vec::new();

                    if let Some(props) = &properties {
                        // FIXME: rename fields to all snake_case and add annotation for rename
                        // also dodge reserved words
                        for (prop_name, prop_type) in props {
                            let mut rename_to = prop_name;
                            let mut prop_name = prop_name.to_snake_case();
                            if prop_name == "type" {
                                prop_name = "type_".to_string();
                            }
                            {
                                context
                                    .extra_name
                                    .borrow_mut()
                                    .push_front(prop_name.clone());
                            }
                            let rust_type =
                                prop_type.schema_object.to_rust_type_name(context.clone())?;
                            let mut field = if let Some(req) = &required
                                && req.contains(&prop_name)
                            {
                                codegen::Field::new(&prop_name, rust_type)
                            } else {
                                codegen::Field::new(&prop_name, format!("Option<{}>", rust_type))
                            };
                            fields.push(if rename_to != &prop_name {
                                field.annotation(format!(r#"serde(rename = "{}")"#, rename_to));
                                field
                            } else {
                                field
                            });
                        }
                    }
                    if let Some(add_props) = &additional_properties {
                        let typea = add_props.schema_object.to_rust_type_name(context.clone())?;
                        fields.push(codegen::Field::new("additional_properties", typea));
                    }
                    {
                        let mut scope = context.scope.borrow_mut();
                        let struc = scope
                            .new_struct(&name.replace(r#"V3."#, ""))
                            .derive("Debug")
                            .derive("Serialize")
                            .derive("Deserialize");
                        for field in fields {
                            struc.push_field(field);
                        }
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

    let paths = result
        .paths
        .iter()
        .map(|(path_name, path)| {
            let mut path_name = path_name.clone();
            // pop of v3
            path_name.elements.pop_front();
            let tl = path_name.elements.pop_front().unwrap();
            (tl, (path_name, path))
        })
        .into_group_map();
    for (path, item) in paths {
        println!(" Path: {:?}", path,);
        for (path_name, path_item) in item {
            for (method, operation) in &path_item.methods {
                let name = if !operation.parameters.path.is_empty() {
                    let rust_type = operation
                        .responses
                        .get("200")
                        .unwrap()
                        .schema
                        .as_ref()
                        .unwrap()
                        .schema_object
                        .to_rust_type_name(context.clone())?
                        .to_snake_case()
                        .replace("_response", "");
                    format!(
                        "{}_{}_by_{}",
                        method,
                        rust_type,
                        operation
                            .parameters
                            .path
                            .iter()
                            .map(|x| &x.name)
                            .join("_and_")
                    )
                } else {
                    format!("{}_{:?}", method, path_name.elements.front())
                };
                let query_params = operation
                    .parameters
                    .path
                    .iter()
                    .map(|param| {
                        let param_name = param.name.to_snake_case();
                        context
                            .extra_name
                            .borrow_mut()
                            .push_front(param_name.clone());
                        let rust_type = param
                            .r#type
                            .schema_object
                            .to_rust_type_name(context.clone())
                            .unwrap();
                        (param_name, rust_type)
                    })
                    .collect_vec();
                let obj_params = operation
                    .parameters
                    .query
                    .iter()
                    .map(|param| {
                        let context = context.clone();
                        let param_name = param.name.to_snake_case();
                        context
                            .extra_name
                            .borrow_mut()
                            .push_front(param_name.clone());
                        let rust_type = param
                            .r#type
                            .schema_object
                            .to_rust_type_name(context)
                            .unwrap();
                        codegen::Field::new(&param_name, format!("Option<{}>", rust_type))
                    })
                    .collect_vec();
                let func_param_name = {
                    let mut scope = context.scope.borrow_mut();

                    let func_params = scope
                        .new_struct(&format!("{}Params", name.to_upper_camel_case()))
                        .derive("Debug")
                        .derive("Serialize")
                        .derive("Default");
                    for field in obj_params {
                        func_params.push_field(field);
                    }
                    func_params.ty().clone()
                };
                let mut scope = context.scope.borrow_mut();
                let mut func = scope
                    .new_fn(&name.to_snake_case())
                    .vis("pub")
                    .ret("Result<(), Error>");
                for (param_name, rust_type) in query_params {
                    func = func.arg(&param_name, &rust_type);
                }
                func.arg("params", func_param_name);
            }
        }
    }

    //    println!("{}", scope.to_string());
    std::fs::write("output.rs", context.scope.borrow().to_string())?;

    Ok(())
}
