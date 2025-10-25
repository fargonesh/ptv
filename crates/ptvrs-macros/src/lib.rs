use proc_macro::TokenStream;
use quote::{quote, ToTokens};

use syn::{
    bracketed,
    parse::{self, discouraged::Speculative, Parse},
    parse_macro_input, parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Expr, FieldValue, Ident, LitStr, Path, Stmt, Token,
};

struct Bracketed<T> {
    _bracket: token::Bracket,
    inner: T,
}

impl<T: Parse> Parse for Bracketed<T> {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _bracket: bracketed!(content in input),
            inner: content.parse()?,
        })
    }
}

enum Grouped2<T> {
    SingleSet(T),
    Set(Bracketed<Punctuated<Bracketed<T>, Token![,]>>),
}
impl<T: Parse> Parse for Grouped2<T> {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        if input.peek(token::Bracket) {
            let content;
            Ok(Self::Set(Bracketed {
                _bracket: bracketed!(content in input ),
                inner: Punctuated::parse_separated_nonempty(&content)?,
            }))
        } else {
            Ok(Self::SingleSet(input.parse()?))
        }
    }
}

struct ExtraParams {
    _comma: Token![,],
    expr: Punctuated<Expr, Token![,]>,
}

impl Parse for ExtraParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _comma = input.parse()?;
        let expr = Punctuated::parse_separated_nonempty(input)?;
        Ok(Self { _comma, expr })
    }
}

struct Field {
    field_name: Ident,
    value: Option<Expr>,
}

impl Parse for Field {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            field_name: input.parse()?,
            value: if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}

enum Grouped<T> {
    Single(T),
    Set {
        bracket: token::Bracket,
        inner: Punctuated<T, Token![,]>,
    },
}

impl<T: Parse> Parse for Grouped<T> {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        if input.peek(token::Bracket) {
            let content;
            Ok(Grouped::Set {
                bracket: bracketed!(content in input),
                inner: Punctuated::parse_separated_nonempty(&content)?,
            })
        } else {
            Ok(Grouped::Single(input.parse()?))
        }
    }
}

struct Struct {
    _comma2: Token![,],
    struc: Path,
    _arrow: Token![=>],
    _bracket: token::Bracket,
    idents: Punctuated<Grouped<Field>, Token![,]>,
}
impl Parse for Struct {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _comma2: input.parse()?,
            struc: input.parse()?,
            _arrow: input.parse()?,
            _bracket: bracketed!(content in input),
            idents: if content.is_empty() {
                Punctuated::new()
            } else {
                Punctuated::parse_separated_nonempty(&content)?
            },
        })
    }
}

struct Params {
    struc: Option<Struct>,
    extraparams: Option<ExtraParams>,
}
impl Parse for Params {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            struc: {
                let fork = input.fork();
                if let Ok(struc) = fork.parse::<Struct>() {
                    input.advance_to(&fork);
                    Some(struc)
                } else {
                    None
                }
            },
            extraparams: {
                let fork = input.fork();
                if let Ok(extraparams) = fork.parse() {
                    input.advance_to(&fork);
                    Some(extraparams)
                } else {
                    None
                }
            },
        })
    }
}

struct MacroInput {
    map: Expr,
    _comma: Token![,],
    name: Ident,
    params: Grouped2<Params>,
}

impl Parse for MacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            map: input.parse()?,
            _comma: input.parse()?,
            name: input.parse()?,
            params: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn make_test(_input: TokenStream) -> TokenStream {
    // parse the input of the macro, which is (Expr, Ident, Ident => [Ident+], Expr+)
    let MacroInput {
        map, name, params, ..
    } = parse_macro_input!(_input as MacroInput);
    let stmts = match params {
        Grouped2::SingleSet(Params { extraparams, struc }) => {
            create_statement_vec(struc, &map, &name, &name.to_string(), extraparams)
        }
        Grouped2::Set(Bracketed { _bracket, inner }) => inner
            .into_iter()
            .flat_map(
                |Bracketed {
                     inner: Params { struc, extraparams },
                     ..
                 }| {
                    let test_name = if let Some(ExtraParams { ref expr, .. }) = extraparams {
                        format!("{}({})", name, expr.to_token_stream())
                    } else {
                        name.to_string()
                    };
                    create_statement_vec(struc, &map, &name, &test_name, extraparams)
                },
            )
            .collect(),
    };

    quote! {
        #(#stmts);*
    }
    .into()
}

fn create_statement_vec(
    struc: Option<Struct>,
    map: &Expr,
    name: &Ident,
    test_name: &str,
    extraparams: Option<ExtraParams>,
) -> Vec<Stmt> {
    let mut stmts: Vec<Stmt> = if let Some(Struct { idents, struc, .. }) = &struc {
        idents
            .iter()
            .map(|group| {
                    let map = &map;
                    // let struc = struc;
                    let extraparams = extraparams.as_ref().map(|x| &x.expr);
                    let (test_name, parsed_struc, span) = match group {
                        Grouped::Single(Field { field_name, value }) => {
                            let test_name = format!("{}::{}", test_name, field_name);
                            let value: Expr  = if let Some(value) = value {
                                parse_quote_spanned!{ value.span() =>
                                    Some(#value)
                                }
                            } else {
                                parse_quote_spanned! { field_name.span() =>
                                    Some(true)
                                }
                            };
                            let parsed_struc: Expr = parse_quote_spanned! { field_name.span() =>
                                #struc {
                                    #field_name: #value,
                                    ..Default::default()
                                }
                            };
                            (test_name, parsed_struc, field_name.span())
                        }
                        Grouped::Set { bracket, inner } => {
                            let mut test_name_vec = vec![format!("{}::",test_name)];
                            let fields = inner
                                .iter()
                                .map(|Field { field_name, value } | -> FieldValue { match value {
                                    Some(value) => {
                                        test_name_vec.push(format!(
                                            "{}({})",
                                            field_name,
                                            value.to_token_stream()
                                        ));
                                        parse_quote_spanned! { value.span() => #field_name: Some(#value) }
                                    }
                                    None => {
                                        test_name_vec.push(field_name.to_string());
                                        parse_quote_spanned! { field_name.span() => #field_name: Some(true)}
                                    }
                                }})
                                .collect::<Punctuated<syn::FieldValue, Token![,]>>();
                            let test_name = test_name_vec.join(",");

                            let parsed_struc: Expr = parse_quote_spanned! { bracket.span =>
                                #struc {
                                    #fields ,
                                    ..Default::default()
                                }};
                            (test_name, parsed_struc, bracket.span.span())
                        }
                    };

                    let params: Punctuated<Expr, Token![,]> = if let Some(expr) = extraparams {
                        parse_quote!(#expr, #parsed_struc)
                    } else {
                        parse_quote!(#parsed_struc )
                    };
                    let test_name: LitStr =  parse_quote! {
                        #test_name
                    };


                    let wow: Stmt = parse_quote_spanned! { span =>
                        #map.insert(#test_name, Arc::new(|| {
                            Box::pin(async {
                                let res = (CLIENT.#name(#params)).await?;
                                Ok(format!("{:?}", res))
                            })
                        }));};

                    wow
            })
            .collect()
    } else {
        vec![]
    };
    let default_params: Punctuated<Expr, Token![,]> = match (extraparams, struc) {
        (Some(ExtraParams { expr, .. }), Some(Struct { struc, .. })) => {
            parse_quote!(#expr, #struc::default())
        }
        (Some(ExtraParams { expr, .. }), None) => {
            parse_quote!(#expr)
        }
        (None, Some(Struct { struc, .. })) => {
            parse_quote!(#struc::default())
        }
        (None, None) => {
            parse_quote!()
        }
    };

    stmts.push(parse_quote! {
        #map.insert(stringify!(#name), Arc::new(|| {
            Box::pin(async {
                let res = (CLIENT.#name(#default_params)).await?;
                Ok(format!("{:?}", res))
            })
        }));
    });
    stmts
}
