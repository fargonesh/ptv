use std::{cell::RefCell, collections::HashMap, rc::Rc, str::FromStr};

use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use quote::quote;
use syn::{DeriveInput, parse::Parse};

use crate::types::{Context, SwaggerFile, ToRustTypeName, TypePath};

struct SwaggerClientArgs {
    path: String,
    strip_prefix: Option<String>,
}

impl Parse for SwaggerClientArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut path = None;
        let mut strip_prefix = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;

            match ident.to_string().as_str() {
                "path" => {
                    let lit: syn::LitStr = input.parse()?;
                    path = Some(lit.value());
                }
                "strip_prefix" => {
                    let lit: syn::LitStr = input.parse()?;
                    strip_prefix = Some(lit.value());
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("Unknown argument: {}", ident),
                    ));
                }
            }

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        let path = path.ok_or_else(|| syn::Error::new(input.span(), "Missing 'path' argument"))?;

        Ok(SwaggerClientArgs { path, strip_prefix })
    }
}

mod types;

fn derive_actual(
    input: DeriveInput,
    args: SwaggerClientArgs,
) -> anyhow::Result<proc_macro::TokenStream> {
    let mut deserializer =
        serde_json::Deserializer::from_reader(std::fs::File::open(&args.path).unwrap());

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
                if let Some(ref prefix) = args.strip_prefix {
                    name.replace(prefix, "")
                } else {
                    name.clone()
                },
            )
        })
        .collect::<std::collections::HashMap<TypePath, String>>();
    let extra_names = vec![("RouteType".to_string(), "ty::RouteType".to_string())]
        .into_iter()
        .collect::<HashMap<_, _>>();
    let context = Rc::new(Context {
        scope: std::cell::RefCell::new(codegen::Scope::new()),
        constant_parameters: HashMap::new(),
        strip_prefix: args.strip_prefix.clone(),
        types: names,
        extra_name: Default::default(),
        extra_types: RefCell::new(extra_names),
    });
    for (name, ty) in &result.definitions {
        let context = context.clone();
        let name = if let Some(ref prefix) = args.strip_prefix {
            name.replace(prefix, "")
        } else {
            name.clone()
        };
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
        //        println!("path_name: {:?}", path_name);
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
                format!("{}_{}", method, path_name.elements.iter().join("_"))
            };
            let _name = context.handle_with_name(name.clone());
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
                    (param_name, rust_type, param.name.clone())
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
                    let mut field =
                        codegen::Field::new(&param_name, format!("Option<{}>", rust_type));
                    field.vis("pub");
                    field
                })
                .collect_vec();
            let func_param_name = {
                let mut scope = context.scope.borrow_mut();

                let func_params = scope
                    .new_struct(&obj_params_name)
                    .vis("pub")
                    .derive("Debug")
                    .derive("Serialize")
                    .derive("Deserialize")
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
            let scope = scope.new_impl(&input.ident.to_string());
            let mut func = scope
                .new_fn(&name.to_snake_case())
                .vis("pub")
                .ret(format!("Result<{},Error>", ret_type));
            func.set_async(true);
            func.arg_ref_self();
            for (param_name, rust_type, _) in path_params.iter() {
                func = func.arg(&param_name, rust_type);
            }
            func.arg("params", func_param_name);
            // Take path parameters and pass them from elements
            // find each `{param}` in path and replace with `{param}` but in snake_case and not using the elements but internal
            let mut path_name = path_name.clone();
            for (param_name, _, original_name) in path_params {
                let to_replace = format!("{{{}}}", original_name);
                let replacement = format!("{{{}}}", param_name);
                path_name.internal = path_name.internal.replace(&to_replace, &replacement);
            }

            func.line(format!("let path = format!(\"{}\");", &path_name.internal));
            func.line("self.rq(format!(\"{}?{}\", path, to_query(params))).await");
        }
    }
    let ident = &input.ident;
    let scope = context.scope.borrow();
    let generated = scope.to_string();
    //    std::fs::write("output.rs", &generated).unwrap();

    Ok(proc_macro::TokenStream::from_str(&generated).unwrap())

    // Further code generation logic would go here...
}

#[proc_macro_derive(SwaggerClient, attributes(swagger))]
pub fn swagger_client_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let args = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("swagger"))
        .expect("Expected a #[swagger(...)] attribute")
        .parse_args::<SwaggerClientArgs>()
        .unwrap();
    derive_actual(input, args).unwrap()
}
