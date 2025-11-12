use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type, parse_macro_input};

#[proc_macro_derive(CorrodeConfig, attributes(form))]
pub fn derive_corrode_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("CorrodeConfig can only be derived for structs with named fields"),
        },
        _ => panic!("CorrodeConfig can only be derived for structs"),
    };

    let field_builders: Vec<_> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let label = format_field_name(&field_name_str);
            let ty: &Type = &field.ty;

            // Determine the field type and generate appropriate builder code
            if is_boolean_type(ty) {
                quote! {
                    (
                        #label.into(),
                        cx.new(|_| {
                            corrode_adapter::form_ui::BooleanField::<#name>::new(
                                #label.into(),
                                |config| config.#field_name,
                                |value, config| config.#field_name = value,
                            )
                        })
                        .into(),
                    )
                }
            } else if is_numeric_type(ty) {
                quote! {
                    (
                        #label.into(),
                        cx.new(|cx| {
                            corrode_adapter::form_ui::NumberField::<#name, _>::new(
                                |config| config.#field_name,
                                |value, config| config.#field_name = value,
                                window,
                                cx,
                            )
                        })
                        .into(),
                    )
                }
            } else {
                quote! {
                    (
                        #label.into(),
                        cx.new(|cx| {
                            corrode_adapter::form_ui::StringField::<#name, _>::new(
                                |config| config.#field_name.clone(),
                                |value, config| config.#field_name = value,
                                window,
                                cx,
                            )
                        })
                        .into(),
                    )
                }
            }
        })
        .collect();

    let expanded = quote! {
        use corrode_adapter::gpui::AppContext;
        impl corrode_adapter::gpui::Global for #name {}
        impl corrode_adapter::config::CorrodeConfig for #name {
            fn generate_form_fields(window: &mut corrode_adapter::gpui::Window, cx: &mut corrode_adapter::gpui::App) -> Vec<(corrode_adapter::gpui::SharedString, corrode_adapter::gpui::AnyView)> where Self: Sized {
                vec![
                    #(#field_builders),*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}

fn format_field_name(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_boolean_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "bool";
        }
    }
    false
}

fn is_numeric_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            return matches!(
                ident.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "f32"
                    | "f64"
            );
        }
    }
    false
}
