use std::borrow::Cow;

use proc_macro2::{TokenStream, Ident, Span};
use syn::{DeriveInput, Data, Fields, DataStruct, FieldsNamed, FieldsUnnamed};
use quote::quote;

fn read_derive_impl(input: &DeriveInput) -> TokenStream {
	let (fields, tuple) = match &input.data {
		Data::Struct(DataStruct { fields: Fields::Named(FieldsNamed { named, .. }), .. }) => (named, false),
		Data::Struct(DataStruct { fields: Fields::Unnamed(FieldsUnnamed { unnamed, .. }), .. }) => (unnamed, true),
		_ => unimplemented!("only tuple struct or struct with named fields supported"),
	};
	let mut body = quote! {};
	let mut initializer = quote! {};
	let mut tuple_field_num = 0u8..;
	for field in fields {
		let mut field_expr = quote! { Readable::read(reader) };
		let mut skip = 0usize;
		let mut zlib = false;
		for attr in &field.attrs {
			if let Some(ident) = attr.path().get_ident() {
				match ident.to_string().as_str() {
					"list_u16" => field_expr = quote! { read_list::<_, _, u16>(reader) },//read a u16, read that many items
					"list_u32" => field_expr = quote! { read_list::<_, _, u32>(reader) },//read a u32, read that many items
					"skip_1" => skip = 1,//skip 1 byte before reading
					"skip_2" => skip = 2,//skip 2 bytes before reading
					"skip_4" => skip = 4,//skip 4 bytes before reading
					"skip_8" => skip = 8,//skip 8 bytes before reading
					"zlib" => zlib = true,//read zlib-compressed item
					_ => {},
				}
			}
		}
		field_expr = quote! { tr_reader::#field_expr? };
		if zlib {
			field_expr = quote! {{
				let reader = &mut tr_reader::get_zlib(reader)?;
				#field_expr
			}};
		}
		if skip > 0 {
			field_expr = quote! {{
				tr_reader::skip::<_, #skip>(reader)?;
				#field_expr
			}};
		}
		let field_ident = match &field.ident {
			Some(field_ident) => Cow::Borrowed(field_ident),
			None => Cow::Owned(Ident::new(&format!("field{}", tuple_field_num.next().unwrap()), Span::call_site())),
		};
		body = quote! {
			#body
			let #field_ident = #field_expr;
		};
		initializer = quote! { #initializer #field_ident, };
	}
	initializer = if tuple { quote! { (#initializer) } } else { quote! { {#initializer} } };
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	let type_name = &input.ident;
	quote! {
		impl #impl_generics tr_reader::Readable for #type_name #ty_generics #where_clause {
			fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
				#body
				Ok(#type_name #initializer)
			}
		}
	}
}

#[proc_macro_derive(
	Readable,
	attributes(
		list_u16,
		list_u32,
		skip_1,
		skip_2,
		skip_4,
		skip_8,
		zlib,
	)
)]
pub fn read_derive(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
	read_derive_impl(&syn::parse_macro_input!(tokens)).into()
}
