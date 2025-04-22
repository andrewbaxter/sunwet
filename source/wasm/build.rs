use {
    convert_case::{
        Case,
        Casing,
    },
    genemichaels_lib::FormatConfig,
    proc_macro2::TokenStream,
    quote::{
        format_ident,
        quote,
    },
    std::{
        env,
        fs::write,
        path::PathBuf,
    },
};

enum TypeMod {
    None,
    Opt,
    Arr,
}

struct Type {
    mod_: TypeMod,
    rust_type: TokenStream,
    ts_type: String,
}

fn type_opt(t: &Type) -> Type {
    return Type {
        mod_: TypeMod::Opt,
        rust_type: t.rust_type.clone(),
        ts_type: t.ts_type.clone(),
    };
}

fn type_arr(t: &Type) -> Type {
    return Type {
        mod_: TypeMod::Arr,
        rust_type: t.rust_type.clone(),
        ts_type: t.ts_type.clone(),
    };
}

struct Func<'a> {
    name: &'a str,
    args: Vec<(&'a str, &'a Type)>,
    returns: Vec<(&'a str, &'a Type)>,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    //. .
    let bool_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(bool),
        ts_type: "boolean".to_string(),
    };
    let usize_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(usize),
        ts_type: "number".to_string(),
    };
    let string_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(&str),
        ts_type: "string".to_string(),
    };
    let arrstring_ = type_arr(&string_);
    let el_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(web_sys::HtmlElement),
        ts_type: "HTMLElement".to_string(),
    };
    let optel_ = type_opt(&el_);
    let arrel_ = type_arr(&el_);
    let promiseel_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(js_sys::Promise),
        ts_type: "Promise<HTMLElement>".to_string(),
    };
    let inpel_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(web_sys::HtmlInputElement),
        ts_type: "HTMLInputElement".to_string(),
    };
    let selel_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(web_sys::HtmlSelectElement),
        ts_type: "HTMLSelectElement".to_string(),
    };
    let any_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(wasm_bindgen::JsValue),
        ts_type: "any".to_string(),
    };

    //. .
    let mut ts = vec![];
    let mut rust = vec![];
    for method in [
        //. .
        Func {
            name: "playlistStateChanged",
            args: vec![("playing", &bool_), ("index", &usize_)],
            returns: vec![],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: all
        Func {
            name: "leafAsyncBlock",
            args: vec![("cb", &promiseel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafErrBlock",
            args: vec![("data", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contTitle",
            args: vec![("left", &el_), ("right", &optel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafTitle",
            args: vec![("text", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contBar",
            args: vec![
                ("extraStyles", &arrstring_),
                ("leftChildren", &arrel_),
                ("leftMidChildren", &arrel_),
                ("midChildren", &arrel_),
                ("rightMidChildren", &arrel_),
                ("rightChildren", &arrel_),
            ],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contBarMainForm",
            args: vec![
                ("leftChildren", &arrel_),
                ("leftMidChildren", &arrel_),
                ("midChildren", &arrel_),
                ("rightMidChildren", &arrel_),
                ("rightChildren", &arrel_),
            ],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contBarMenu",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contSpinner",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafSpace",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafButton",
            args: vec![("title", &string_), ("text", &string_), ("extraStyles", &arrstring_)],
            returns: vec![("root", &el_), ("button", &el_)],
        },
        Func {
            name: "leafBarButtonBig",
            args: vec![("title", &string_), ("icon", &string_)],
            returns: vec![("root", &el_), ("button", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, form + edit
        Func {
            name: "leafInputPair",
            args: vec![("label", &string_), ("inputId", &string_), ("input", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafInputText",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_), ("input", &el_)],
        },
        Func {
            name: "leafInputPairText",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_), ("input", &el_)],
        },
        Func {
            name: "leafInputSelect",
            args: vec![("id", &string_), ("children", &arrel_)],
            returns: vec![("root", &el_), ("input", &selel_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, view
        Func {
            name: "contPageView",
            args: vec![("entries", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contBarViewTransport",
            args: vec![],
            returns: vec![
                ("root", &el_),
                ("buttonShare", &el_),
                ("buttonPrev", &el_),
                ("buttonNext", &el_),
                ("buttonPlay", &el_),
                ("seekbar", &el_),
                ("seekbarFill", &el_),
            ],
        },
        Func {
            name: "contMediaFullscreen",
            args: vec![("media", &el_)],
            returns: vec![("root", &el_), ("buttonClose", &el_)],
        },
        Func {
            name: "contModal",
            args: vec![("title", &string_), ("child", &el_)],
            returns: vec![("root", &el_), ("buttonClose", &el_)],
        },
        Func {
            name: "leafTransportButton",
            args: vec![("title", &string_), ("icon", &string_)],
            returns: vec![("root", &el_), ("button", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, form
        Func {
            name: "contPageForm",
            args: vec![("entries", &arrel_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, edit
        Func {
            name: "contPageEdit",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contPageEditSectionRel",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafButtonEditFree",
            args: vec![("icon", &string_), ("hint", &string_)],
            returns: vec![("root", &el_), ("button", &el_)],
        },
        Func {
            name: "leafEditNode",
            args: vec![("id", &string_), ("nodeHint", &string_), ("nodeType", &string_), ("node", &string_)],
            returns: vec![
                ("root", &el_),
                ("inputType", &selel_),
                ("inputValue", &inpel_),
                ("buttonDelete", &el_),
                ("buttonRevert", &el_),
            ],
        },
        Func {
            name: "leafEditPredicate",
            args: vec![("id", &string_), ("value", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafEditRowIncoming",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafEditRowOutgoing",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: menu
        Func {
            name: "contBodyMenu",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMenuGroup",
            args: vec![("title", &string_), ("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuLink",
            args: vec![("title", &string_), ("href", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: Main
        Func {
            name: "appMain",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx PLUGINS: View
        Func {
            name: "buildView",
            args: vec![("pluginId", &string_), ("arguments", &any_)],
            returns: vec![("root", &promiseel_)],
        },
    ] {
        let method_ts_name = method.name;

        // Ts side
        ts.push(format!("    {}: (args: {{ {} }}) => {};", method_ts_name, {
            let mut spec = vec![];
            for (ts_name, type_) in &method.args {
                match type_.mod_ {
                    TypeMod::None => {
                        spec.push(format!("{}: {}", ts_name, type_.ts_type))
                    },
                    TypeMod::Opt => {
                        spec.push(format!("{}?: {}", ts_name, type_.ts_type))
                    },
                    TypeMod::Arr => {
                        spec.push(format!("{}: {}[]", ts_name, type_.ts_type))
                    },
                }
            }
            spec.join(", ")
        }, if method.returns.is_empty() {
            "void".to_string()
        } else {
            let mut spec = vec![];
            for (ts_name, type_) in &method.returns {
                match type_.mod_ {
                    TypeMod::None => {
                        spec.push(format!("{}: {}", ts_name, type_.ts_type))
                    },
                    TypeMod::Opt => {
                        spec.push(format!("{}?: {}", ts_name, type_.ts_type))
                    },
                    TypeMod::Arr => {
                        spec.push(format!("{}: {}[]", ts_name, type_.ts_type))
                    },
                }
            }
            format!("{{ {} }}", spec.join(", "))
        }));

        // Rust side
        let rust_name = format_ident!("{}", method.name.to_case(Case::Snake));
        let rust_args = format_ident!("{}Args", method.name.to_case(Case::UpperCamel));
        let mut rust_args_spec = vec![];
        let mut rust_args_build = vec![];
        for (ts_name, type_) in &method.args {
            let rust_name = format_ident!("{}", ts_name.to_case(Case::Snake));
            let rust_default;
            let rust_type;
            match type_.mod_ {
                TypeMod::None => {
                    rust_default = quote!();
                    rust_type = type_.rust_type.clone();
                },
                TypeMod::Opt => {
                    rust_default = quote!(#[default]);
                    let rust_type1 = &type_.rust_type;
                    rust_type = quote!(Option <#rust_type1 >);
                },
                TypeMod::Arr => {
                    rust_default = quote!(#[default]);
                    let rust_type1 = &type_.rust_type;
                    rust_type = quote!(Vec <#rust_type1 >);
                },
            };
            rust_args_spec.push(quote!(#rust_default pub #rust_name: #rust_type));
            rust_args_build.push(quote!{
                js_set(&a, #ts_name, & args.#rust_name);
            });
        }
        let rust_ret = format_ident!("{}Ret", method.name.to_case(Case::UpperCamel));
        let mut rust_ret_spec = vec![];
        let mut rust_ret_build = vec![];
        for (ts_name, type_) in &method.returns {
            let rust_name = format_ident!("{}", ts_name.to_case(Case::Snake));
            let rust_default;
            let rust_type;
            match type_.mod_ {
                TypeMod::None => {
                    rust_default = quote!();
                    rust_type = type_.rust_type.clone();
                },
                TypeMod::Opt => {
                    rust_default = quote!(#[default]);
                    let rust_type1 = &type_.rust_type;
                    rust_type = quote!(Option <#rust_type1 >);
                },
                TypeMod::Arr => {
                    rust_default = quote!(#[default]);
                    let rust_type1 = &type_.rust_type;
                    rust_type = quote!(Vec <#rust_type1 >);
                },
            };
            rust_ret_spec.push(quote!(#rust_default pub #rust_name: #rust_type));
            rust_ret_build.push(quote!{
                #rust_name: js_get(ret, #ts_name)
            });
        }
        rust.push(quote!{
            #[derive(Default)] struct #rust_args {
                #(#rust_args_spec,) *
            }
            struct #rust_ret {
                #(#rust_ret_spec,) *
            }
            fn #rust_name(args:#rust_args) -> #rust_ret {
                let a = js_sys::Object::new();
                //. .
                #(#rust_args_build) * 
                //. .
                let _ret = js_call(& js_get(&js_get(&gloo::utils::window().into(), "sunwet_presentation"), #method_ts_name), &a);
                return #rust_ret {
                    #(#rust_ret_build,) *
                };
            }
        });
    }
    write(PathBuf::from(&env::var("OUT_DIR").unwrap()).join("style_export.rs"), genemichaels_lib::format_str(&quote!{
        #(#rust) *
    }.to_string(), &FormatConfig::default()).unwrap().rendered).unwrap();
    write(
        PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("static/style_export3.d.ts"),
        format!("// Generated by build.rs\ndeclare type Presentation = {{\n{}\n}};", ts.join("\n")),
    ).unwrap();
}
