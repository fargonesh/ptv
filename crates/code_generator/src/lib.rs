use anyhow::Context as AnyhowContext;
use std::{collections::HashMap, rc::Rc, str::FromStr};

use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use syn::{DeriveInput, parse::Parse, spanned::Spanned};

use crate::types::{Context, SwaggerFile, ToRustTypeName, TypePath};

struct SwaggerClientArgs {
    path: String,
    strip_prefix: Option<String>,
    skipped: Vec<String>,
    extra_names: HashMap<String, String>,
}

impl Parse for SwaggerClientArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut path = None;
        let mut strip_prefix = None;
        let mut extra_names = None;
        let mut skipped = Vec::new();

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
                "skip" => {
                    let skips: syn::ExprArray = input.parse()?;
                    for expr in skips.elems.iter() {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit),
                            ..
                        }) = expr
                        {
                            skipped.push(lit.value());
                        } else {
                            return Err(syn::Error::new(
                                expr.span(),
                                "Expected string literals in skip array",
                            ));
                        }
                    }

                    // Ignore the value for now
                }
                "extra_names" => {
                    let map: syn::ExprArray = input.parse()?;
                    let mut names_map = HashMap::new();
                    for expr in map.elems.iter() {
                        if let syn::Expr::Tuple(tuple) = expr {
                            if tuple.elems.len() == 2 {
                                if let (
                                    syn::Expr::Lit(syn::ExprLit {
                                        lit: syn::Lit::Str(lit1),
                                        ..
                                    }),
                                    syn::Expr::Lit(syn::ExprLit {
                                        lit: syn::Lit::Str(lit2),
                                        ..
                                    }),
                                ) = (&tuple.elems[0], &tuple.elems[1])
                                {
                                    names_map.insert(lit1.value(), lit2.value());
                                } else {
                                    return Err(syn::Error::new(
                                        tuple.span(),
                                        "Expected string literals in extra_names tuples",
                                    ));
                                }
                            } else {
                                return Err(syn::Error::new(
                                    tuple.span(),
                                    "Expected tuples of length 2 in extra_names",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new(
                                expr.span(),
                                "Expected tuples in extra_names array",
                            ));
                        }
                    }
                    extra_names = Some(names_map);
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

        Ok(SwaggerClientArgs {
            path,
            strip_prefix,
            skipped,
            extra_names: extra_names.unwrap_or_default(),
        })
    }
}

#[macro_use]
mod types;

fn derive_actual(
    input: DeriveInput,
    args: SwaggerClientArgs,
) -> anyhow::Result<proc_macro::TokenStream> {
    let mut deserializer =
        serde_json::Deserializer::from_reader(std::fs::File::open(&args.path).unwrap());
    let mut constant_parameters = Vec::new();
    constant_parameters.extend(args.skipped);

    if let syn::Data::Struct(strukt) = &input.data {
        strukt.fields.iter().for_each(|field| {
            let field_name = field.ident.as_ref().unwrap().to_string();
            field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("swagger"));
            constant_parameters.push(field_name);
        });
    }
    println!("Constant parameters: {:?}", &constant_parameters);
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

    let context = Rc::new(Context {
        scope: std::cell::RefCell::new(codegen::Scope::new()),
        constant_parameters,
        strip_prefix: args.strip_prefix.clone(),
        types: names,
        name_stack: Default::default(),
        extra_types: args.extra_names,
    });
    let mut module = codegen::Module::new("generated_types");
    {
        let mut scope = context.scope.borrow_mut();
        module.import("serde", "Serialize");
        module.import("serde", "Deserialize");
        module.import("derive_more", "Display");
        module.vis("pub");
        scope.push_module(module.clone());
    }
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
            let ret_type = operation
                .responses
                .get("200")
                .unwrap()
                .schema
                .as_ref()
                .unwrap()
                .schema_object
                .to_rust_type_name(context.clone())?;

            let name = if !operation.parameters.path.is_empty() {
                let rust_type = ret_type.to_snake_case().replace("_response", "");
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
                .filter(|param| !context.constant_parameters.contains(&param.name))
                .map(|param| {
                    let param_name = param.name.to_snake_case();
                    let _name_handle = context.handle_with_name(param_name.clone());
                    let rust_type = if let Some(ty) =
                        context.extra_types.get(&param_name.to_upper_camel_case())
                    {
                        ty.clone()
                    } else {
                        param
                            .r#type
                            .schema_object
                            .to_rust_type_name(context.clone())
                            .unwrap()
                    };

                    (param_name, rust_type, param.name.clone())
                })
                .collect_vec();
            let obj_params_name = format!("{}Params", name.to_upper_camel_case());
            let _handle = context.handle_with_name(obj_params_name.clone());
            let obj_params = operation
                .parameters
                .query
                .iter()
                .filter(|param| !context.constant_parameters.contains(&param.name))
                .map(|param| {
                    let context = context.clone();
                    let param_name = param.name.to_snake_case();
                    let _handle = context.handle_with_name(param_name.clone());
                    let rust_type = if let Some(ty) =
                        context.extra_types.get(&param_name.to_upper_camel_case())
                    {
                        ty.clone()
                    } else {
                        param
                            .r#type
                            .schema_object
                            .to_rust_type_name(context.clone())
                            .unwrap()
                    };
                    let mut field =
                        codegen::Field::new(&param_name, format!("Option<{}>", rust_type));
                    field.vis("pub");
                    if let Some(ref docs) = param.description {
                        field.doc(docs);
                    }
                    field.annotation("#[serde(skip_serializing_if = \"Option::is_none\")]");
                    field
                })
                .collect_vec();
            let func_param_name = {
                if obj_params.is_empty() {
                    None
                } else {
                    context!(context, scope);

                    let func_params = scope
                        .new_struct(&obj_params_name)
                        .vis("pub")
                        .derive("Default");
                    struc_opts!(func_params);
                    for field in obj_params {
                        func_params.push_field(field);
                    }
                    Some(obj_params_name)
                }
            };

            let ret_type = format!(
                "{}::{}",
                {
                    context!(context, scope);
                    scope.name.clone()
                },
                ret_type.to_upper_camel_case()
            );

            let mut scope = context.scope.borrow_mut();
            let scope = scope.new_impl(&input.ident.to_string());
            let mut func = scope
                .new_fn(&name.to_snake_case())
                .vis("pub")
                .ret(format!("Result<{},Error>", ret_type));
            if let Some(ref docs) = operation.summary {
                func.doc(docs);
            }
            func.set_async(true);
            func.arg_ref_self();
            for (param_name, rust_type, _) in path_params.iter() {
                func = func.arg(
                    &param_name,
                    if rust_type == "String" {
                        "impl AsRef<str>"
                    } else {
                        &rust_type
                    },
                );
            }
            if let Some(func_param_name) = &func_param_name {
                func.arg(
                    "params",
                    format!("{}::{}", "generated_types", func_param_name),
                );
            }
            let mut path_name = path_name.clone();
            for (param_name, ty, original_name) in path_params {
                let to_replace = format!("{{{}}}", original_name);
                let replacement = if ty == "String" {
                    func.line(format!(
                        "let {0} =  url_escape::encode_path(&clean({0}.as_ref().to_string())).into_owned();",
                        &param_name
                    ));
                    format!("{{{}}}", param_name)
                } else {
                    format!("{{{}}}", param_name)
                };
                path_name.internal = path_name.internal.replace(&to_replace, &replacement);
            }
            //            println!("Generating function a: {}", path_name.internal);

            func.line(format!("let path = format!(\"{}\");", &path_name.internal));
            if let Some(_params) = func_param_name {
                func.line("self.rq(format!(\"{}?{}\", path, to_query(params))).await");
            } else {
                func.line("self.rq(path).await");
            }
        }
    }
    let _ident = &input.ident;
    let scope = context.scope.borrow_mut();
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
