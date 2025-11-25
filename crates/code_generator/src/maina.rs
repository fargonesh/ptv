use std::{collections::HashMap, rc::Rc};

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
        constant_parameters: HashMap::new(),
        extra_types: std::collections::HashMap::new().into(),
        strip_prefix: Default::default(),
        types: names,
        extra_name: Default::default(),
    });
    {
        let mut scope = context.scope.borrow_mut();
        scope
            .new_enum("RouteTypeEnum")
            .derive("Debug")
            .derive("Serialize")
            .derive("Deserialize")
            .new_variant("Lol");
        context
            .extra_types
            .borrow_mut()
            .insert("RouteType".to_string(), "RouteTypeEnum".to_string());
    }

    for (name, ty) in &result.definitions {
        let context = context.clone();
        let name = name.replace(r#"V3."#, "");
        let _handle = context.handle_with_name(name.clone());
        ty.schema_object.to_rust_type_name(context.clone())?;
    }
    let paths = result
        .paths
        .into_iter()
        .map(|(mut k, v)| {
            k.elements.pop_front();
            (k, v)
        })
        .collect::<HashMap<_, _>>();

    for (path_name, path_item) in paths {
        println!("path_name: {:?}", path_name);
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
                format!("{}_{}", method, path_name.elements.front().unwrap())
            };
            let path_params = operation
                .parameters
                .path
                .iter()
                .map(|param| {
                    let param_name = param.name.to_snake_case();
                    let _name_handle = context.handle_with_name(param_name.clone());
                    let rust_type = param
                        .r#type
                        .schema_object
                        .to_rust_type_name(context.clone())
                        .unwrap();
                    (param_name, rust_type)
                })
                .collect_vec();
            let obj_params_name = format!("{}Params", name.to_upper_camel_case());
            let _handle = context.handle_with_name(obj_params_name.clone());
            let obj_params = operation
                .parameters
                .query
                .iter()
                .map(|param| {
                    let context = context.clone();
                    let param_name = param.name.to_snake_case();
                    let _handle = context.handle_with_name(param_name.clone());
                    let rust_type = param
                        .r#type
                        .schema_object
                        .to_rust_type_name(context.clone())
                        .unwrap();
                    codegen::Field::new(&param_name, format!("Option<{}>", rust_type))
                })
                .collect_vec();
            let func_param_name = {
                let mut scope = context.scope.borrow_mut();

                let func_params = scope
                    .new_struct(&obj_params_name)
                    .derive("Debug")
                    .derive("Serialize")
                    .derive("Default");
                for field in obj_params {
                    func_params.push_field(field);
                }
                func_params.ty().clone()
            };

            let ret_type = operation
                .responses
                .get("200")
                .unwrap()
                .schema
                .as_ref()
                .unwrap()
                .schema_object
                .to_rust_type_name(context.clone())?;

            let mut scope = context.scope.borrow_mut();
            let mut func = scope
                .new_fn(&name.to_snake_case())
                .vis("pub")
                .ret(format!("Result<{},Error>", ret_type));
            for (param_name, rust_type) in path_params {
                func = func.arg(&param_name, &rust_type);
            }
            func.arg("params", func_param_name);
        }
    }

    //    println!("{}", scope.to_string());
    std::fs::write("output.rs", context.scope.borrow().to_string())?;

    Ok(())
}
