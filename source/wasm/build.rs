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
    Arr2,
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

fn type_arr2(t: &Type) -> Type {
    return Type {
        mod_: TypeMod::Arr2,
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
    let string_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(String),
        ts_type: "string".to_string(),
    };
    let arrstring_ = type_arr(&string_);
    let optstring_ = type_opt(&string_);
    let el_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(web_sys::Element),
        ts_type: "Element".to_string(),
    };
    let optel_ = type_opt(&el_);
    let arrel_ = type_arr(&el_);
    let arrarrel_ = type_arr2(&el_);
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
    let strmap = Type {
        mod_: TypeMod::None,
        rust_type: quote!(std::collections::HashMap < String, String >),
        ts_type: "{[k: string]: string}".to_string(),
    };
    let direction = Type {
        mod_: TypeMod::None,
        rust_type: quote!(Direction),
        ts_type: "Direction".to_string(),
    };
    let orientation = Type {
        mod_: TypeMod::None,
        rust_type: quote!(Orientation),
        ts_type: "Orientation".to_string(),
    };
    let transalign = Type {
        mod_: TypeMod::None,
        rust_type: quote!(TransAlign),
        ts_type: "TransAlign".to_string(),
    };

    //. .
    let mut ts = vec![];
    let mut rust = vec![];
    for method in [
        Func {
            name: "classMenuWantStateOpen",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classMenuStateOpen",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateHide",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateThinking",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateModified",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateInvalid",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateDeleted",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStatePlaying",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateSharing",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: all
        Func {
            name: "contGroup",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contStack",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafAsyncBlock",
            args: vec![("inRoot", &bool_), ("extraStyles", &arrstring_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafErrBlock",
            args: vec![("data", &string_), ("inRoot", &bool_)],
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
            name: "leafSpinner",
            args: vec![("extraStyles", &arrstring_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafSpace",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafButtonBig",
            args: vec![("title", &string_), ("icon", &optstring_), ("text", &optstring_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafButtonBigSave",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafButtonSmallEdit",
            args: vec![("link", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafButtonSmallView",
            args: vec![("link", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: home
        Func {
            name: "contPageHome",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, form + edit
        Func {
            name: "leafInputText",
            args: vec![("id", &optstring_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafInputNumber",
            args: vec![("id", &optstring_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &inpel_)],
        },
        Func {
            name: "leafInputBool",
            args: vec![("id", &optstring_), ("title", &string_), ("value", &bool_)],
            returns: vec![("root", &inpel_)],
        },
        Func {
            name: "leafInputDate",
            args: vec![("id", &optstring_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &inpel_)],
        },
        Func {
            name: "leafInputTime",
            args: vec![("id", &optstring_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &inpel_)],
        },
        Func {
            name: "leafInputDatetime",
            args: vec![("id", &optstring_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &inpel_)],
        },
        Func {
            name: "leafInputColor",
            args: vec![("id", &optstring_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &inpel_)],
        },
        Func {
            name: "leafInputEnum",
            args: vec![("id", &optstring_), ("title", &string_), ("options", &strmap), ("value", &string_)],
            returns: vec![("root", &selel_)],
        },
        Func {
            name: "leafInputPair",
            args: vec![("label", &string_), ("inputId", &string_), ("input", &el_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafInputPairText",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_), ("input", &el_)],
        },
        Func {
            name: "leafInputPairNumber",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_), ("input", &inpel_)],
        },
        Func {
            name: "leafInputPairBool",
            args: vec![("id", &string_), ("title", &string_), ("value", &bool_)],
            returns: vec![("root", &el_), ("input", &inpel_)],
        },
        Func {
            name: "leafInputPairDate",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_), ("input", &inpel_)],
        },
        Func {
            name: "leafInputPairTime",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_), ("input", &inpel_)],
        },
        Func {
            name: "leafInputPairDatetime",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_), ("input", &inpel_)],
        },
        Func {
            name: "leafInputPairColor",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_)],
            returns: vec![("root", &el_), ("input", &inpel_)],
        },
        Func {
            name: "leafInputPairEnum",
            args: vec![("id", &string_), ("title", &string_), ("value", &string_), ("options", &strmap)],
            returns: vec![("root", &el_), ("input", &selel_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, view
        Func {
            name: "contPageView",
            args: vec![("transport", &optel_), ("rows", &el_)],
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
                ("seekbarLabel", &el_)
            ],
        },
        Func {
            name: "contViewRootRows",
            args: vec![("rows", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contViewRow",
            args: vec![("blocks", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contViewBlock",
            args: vec![("children", &arrel_), ("width", &optstring_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMediaFullscreen",
            args: vec![("media", &el_)],
            returns: vec![("root", &el_), ("buttonClose", &el_)],
        },
        Func {
            name: "contModalViewShare",
            args: vec![("qr", &el_), ("link", &string_)],
            returns: vec![("root", &el_), ("bg", &el_), ("buttonClose", &el_), ("buttonUnshare", &el_)],
        },
        Func {
            name: "contViewList",
            args: vec![
                ("direction", &direction),
                ("xScroll", &bool_),
                ("children", &arrel_),
                ("gap", &optstring_)
            ],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contViewTable",
            args: vec![
                ("orientation", &orientation),
                ("xScroll", &bool_),
                ("children", &arrarrel_),
                ("gap", &optstring_)
            ],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafViewImage",
            args: vec![
                ("transAlign", &transalign),
                ("src", &string_),
                ("link", &optstring_),
                ("text", &optstring_),
                ("width", &optstring_),
                ("height", &optstring_)
            ],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafViewText",
            args: vec![
                ("transAlign", &transalign),
                ("orientation", &orientation),
                ("text", &string_),
                ("fontSize", &optstring_),
                ("maxSize", &optstring_),
                ("link", &optstring_)
            ],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafViewPlayButton",
            args: vec![("transAlign", &transalign), ("direction", &direction)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, form
        Func {
            name: "contPageForm",
            args: vec![("entries", &arrel_), ("barChildren", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafFormComment",
            args: vec![("text", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, view/edit node
        Func {
            name: "contPageNodeSectionRel",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contNodeRowIncoming",
            args: vec![("children", &arrel_), ("new", &bool_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contNodeRowOutgoing",
            args: vec![("children", &arrel_), ("new", &bool_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contNodeSectionCenter",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, node view
        Func {
            name: "contPageNodeView",
            args: vec![("pageButtonChildren", &arrel_), ("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafNodeViewPredicate",
            args: vec![("value", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafNodeViewNodeText",
            args: vec![("value", &string_), ("link", &optstring_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: page, node edit
        Func {
            name: "contPageNodeEdit",
            args: vec![("pageButtonChildren", &arrel_), ("barChildren", &arrel_), ("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafButtonNodeEditAdd",
            args: vec![("hint", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafNodeEditButtons",
            args: vec![],
            returns: vec![("root", &el_), ("buttonDelete", &el_), ("buttonRevert", &el_)],
        },
        Func {
            name: "leafNodeEditNode",
            args: vec![("inputType", &el_), ("inputValue", &el_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafNodeEditPredicate",
            args: vec![("value", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: menu
        Func {
            name: "contBarMenu",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuBarButtonLogin",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuBarButtonLogout",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMenuBody",
            args: vec![("children", &arrel_), ("user", &string_), ("barChildren", &arrel_)],
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
            args: vec![("mainTitle", &el_), ("mainBody", &el_), ("menuBody", &el_)],
            returns: vec![("root", &el_), ("admenuButton", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: Link
        Func {
            name: "appLinkPerms",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "appLink",
            args: vec![],
            returns: vec![
                ("root", &el_),
                ("displayUnder", &el_),
                ("display", &el_),
                ("displayOver", &el_),
                ("albumArtist", &el_),
                ("title", &el_)
            ],
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
                    TypeMod::Arr2 => {
                        spec.push(format!("{}: {}[][]", ts_name, type_.ts_type))
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
                    TypeMod::Arr2 => {
                        spec.push(format!("{}: {}[][]", ts_name, type_.ts_type))
                    },
                }
            }
            format!("{{ {} }}", spec.join(", "))
        }));

        // Rust side
        let rust_name = format_ident!("{}", method.name.to_case(Case::Snake));
        let rust_args_struct_declare;
        let rust_args_declare;
        let rust_args_build;
        if method.args.is_empty() {
            rust_args_struct_declare = quote!();
            rust_args_declare = quote!();
            rust_args_build = quote!();
        } else {
            let args_ident = format_ident!("{}Args", method.name.to_case(Case::UpperCamel));
            let mut spec = vec![];
            let mut build = vec![];
            for (ts_name, type_) in &method.args {
                let rust_name = format_ident!("{}", ts_name.to_case(Case::Snake));
                let rust_type;
                match type_.mod_ {
                    TypeMod::None => {
                        rust_type = type_.rust_type.clone();
                    },
                    TypeMod::Opt => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Option <#rust_type1 >);
                    },
                    TypeMod::Arr => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Vec <#rust_type1 >);
                    },
                    TypeMod::Arr2 => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Vec < Vec <#rust_type1 >>);
                    },
                };
                spec.push(quote!(pub #rust_name: #rust_type));
                build.push(quote!{
                    js_set(&a, #ts_name, & args.#rust_name);
                });
            }
            rust_args_struct_declare = quote!{
                pub struct #args_ident {
                    #(#spec,) *
                }
            };
            rust_args_declare = quote!(args:#args_ident);
            rust_args_build = quote!{
                #(#build) *
            };
        }
        let call =
            quote!(
                js_call(& js_get(&js_get(&gloo::utils::window().into(), "sunwetPresentation"), #method_ts_name), &a);
            );
        let rust_ret;
        let rust_ret_struct_declare;
        let call1;
        if method.returns.is_empty() {
            rust_ret_struct_declare = quote!();
            rust_ret = quote!(());
            call1 = quote!{
                #call
            };
        } else {
            let ident = format_ident!("{}Ret", method.name.to_case(Case::UpperCamel));
            let mut spec = vec![];
            let mut build = vec![];
            for (ts_name, type_) in &method.returns {
                let rust_name = format_ident!("{}", ts_name.to_case(Case::Snake));
                let rust_type;
                match type_.mod_ {
                    TypeMod::None => {
                        rust_type = type_.rust_type.clone();
                    },
                    TypeMod::Opt => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Option <#rust_type1 >);
                    },
                    TypeMod::Arr => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Vec <#rust_type1 >);
                    },
                    TypeMod::Arr2 => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Vec < Vec <#rust_type1 >>);
                    },
                };
                spec.push(quote!(pub #rust_name: #rust_type));
                build.push(quote!{
                    #rust_name: js_get(&_ret, #ts_name)
                });
            }
            call1 = quote!{
                let _ret = #call 
                //. .
                return #ident {
                    #(#build,) *
                };
            };
            rust_ret = quote!(#ident);
            rust_ret_struct_declare = quote!{
                pub struct #ident {
                    #(#spec,) *
                }
            };
        }
        rust.push(quote!{
            #rust_args_struct_declare 
            //. .
            #rust_ret_struct_declare 
            //. .
            pub fn #rust_name(#rust_args_declare) -> #rust_ret {
                let a = js_sys::Object::new();
                //. .
                #rust_args_build 
                //. .
                #call1
            }
        });
    }
    write(PathBuf::from(&env::var("OUT_DIR").unwrap()).join("style_export.rs"), genemichaels_lib::format_str(&quote!{
        #(#rust) *
    }.to_string(), &FormatConfig::default()).unwrap().rendered).unwrap();
    write(
        PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("static/style_export.d.ts"),
        format!("// Generated by build.rs\ndeclare type Presentation = {{\n{}\n}};", ts.join("\n")),
    ).unwrap();
}
