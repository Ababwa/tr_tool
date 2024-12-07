mod readable;

#[proc_macro_derive(Readable, attributes(boxed, zlib, delegate, list, save_pos, seek))]
pub fn derive_readable(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
	readable::derive_readable_impl(syn::parse_macro_input!(item)).into()
}
