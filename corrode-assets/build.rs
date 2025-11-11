use quote::{format_ident, quote};
use std::path::Path;
use std::{env, fs};

include!("src/assets.rs");

fn main() {
    println!("cargo:rerun-if-changed=assets/");

    let (name, path): (Vec<_>, Vec<_>) = CorrodeAssetSource::iter()
        .into_iter()
        .filter_map(|asset| {
            let name = asset.split_once("/")?.1.split_once(".")?.0;
            let name = name
                .split("-")
                .map(|part| format!("{}{}", part[0..1].to_uppercase(), &part[1..]))
                .collect::<String>();
            let name = format_ident!("{}", name);
            let path = quote! { Self::#name => #asset };
            Some((name, path))
        })
        .unzip();

    let output = quote! {
        use gpui::{AnyElement, App, IntoElement, RenderOnce, SharedString, Window};
        use gpui_component::Icon;

        #[derive(IntoElement, Clone)]
        pub enum Assets {
            #(#name),*
        }

        impl Assets {
            pub fn path(self) -> SharedString {
                match self {
                    #(#path),*
                }.into()
            }
        }

        impl From<Assets> for Icon {
            fn from(asset: Assets) -> Self {
                Icon::default().path(asset.path())
            }
        }

        impl From<Assets> for AnyElement {
            fn from(asset: Assets) -> Self {
                Icon::from(asset).into_any_element()
            }
        }

        impl RenderOnce for Assets {
            fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
                Icon::from(self)
            }
        }
    };

    let syntax_tree = syn::parse2(output).unwrap();
    let formatted = prettyplease::unparse(&syntax_tree);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("out.rs");
    fs::write(dest_path, formatted).unwrap();
}
