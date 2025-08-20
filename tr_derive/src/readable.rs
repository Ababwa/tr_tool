use proc_macro2::TokenStream;
use quote::quote;
use syn::{
	parse::Parser, parse2, punctuated::Punctuated, Data, DataStruct, DeriveInput, Error, Field, Fields,
	FieldsNamed, Ident, Meta, MetaList, Path, Token,
};

trait AttrReceiver: Sized {
	fn get(meta: Meta) -> Result<Self, Option<Error>>;
}

//#[attr]
impl AttrReceiver for bool {
	fn get(meta: Meta) -> Result<Self, Option<Error>> {
		match meta {
			Meta::Path(_) => Ok(true),
			_ => Err(None),
		}
	}
}

//#[attr(ident)]
impl AttrReceiver for Option<Ident> {
	fn get(meta: Meta) -> Result<Self, Option<Error>> {
		match meta {
			Meta::List(MetaList { tokens, .. }) => match parse2(tokens) {
				Ok(ident) => Ok(Some(ident)),
				Err(e) => Err(Some(e)),
			},
			_ => Err(None),
		}
	}
}

//#[attr(ident1, ident2, ..)]
impl AttrReceiver for Option<Vec<Ident>> {
	fn get(meta: Meta) -> Result<Self, Option<Error>> {
		match meta {
			Meta::Path(_) => Ok(None),
			Meta::List(MetaList { tokens, .. }) => match Punctuated::<Ident, Token![,]>::parse_terminated.parse2(tokens) {
				Ok(iter) => Ok(Some(iter.into_iter().collect())),
				Err(e) => Err(Some(e)),
			},
			_ => Err(None),
		}
	}
}

//#[attr] or #[attr(path1, path2, ..)]
impl AttrReceiver for Option<Option<Vec<Path>>> {
	fn get(meta: Meta) -> Result<Self, Option<Error>> {
		match meta {
			Meta::Path(_) => Ok(Some(None)),
			Meta::List(MetaList { tokens, .. }) => match Punctuated::<Path, Token![,]>::parse_terminated.parse2(tokens) {
				Ok(iter) => Ok(Some(Some(iter.into_iter().collect()))),
				Err(e) => Err(Some(e)),
			},
			_ => Err(None),
		}
	}
}

macro_rules! parse_attrs_fn {
	(
		$fn_name:ident -> $type_name:ident {
			$($attr_name:ident: $attr_type:ty,)*
		}
	) => {
		struct $type_name {
			$($attr_name: $attr_type,)*
		}
		
		fn $fn_name(attrs: Vec<syn::Attribute>) -> Result<$type_name, String> {
			$(let mut $attr_name = None;)*
			for attr in attrs {
				match attr.path().get_ident().expect("attribute ident").to_string().as_str() {
					$(
						stringify!($attr_name) => {
							if $attr_name.is_some() {
								return Err(format!("cannot use helper attribute more than once: {}", stringify!($attr_name)));
							}
							match <$attr_type as AttrReceiver>::get(attr.meta) {
								Ok(val) => $attr_name = Some(val),
								Err(Some(e)) => return Err(format!("{}: {}", stringify!($attr_name), e)),
								Err(None) => return Err(format!("invalid helper attribute form: {}", stringify!($attr_name))),
							}
						},
					)*
					_ => {},
				}
			}
			Ok($type_name { $($attr_name: $attr_name.unwrap_or_default()),* })
		}
	};
}

parse_attrs_fn!(
	parse_field_attrs -> FieldAttrs {
		boxed: bool,
		zlib: bool,
		list: Option<Ident>,
		delegate: Option<Option<Vec<Path>>>,
		save_pos: Option<Ident>,
		seek: Option<Vec<Ident>>,
	}
);

fn get_delegate_init(delegate_args: Option<Vec<Path>>, ptr: TokenStream, initialized_fields: &[Ident], saved_positions: &[Ident]) -> Result<TokenStream, String> {
	let (func, args) = match delegate_args {
		None => (quote! { tr_readable::Readable::read }, quote! {}),
		Some(paths) => match &paths[..] {
			[] => return Err("parameterized form of `delegate` must include at least one argument".to_string()),
			[func, ..] => {
				let mut args = quote! {};
				for arg in &paths[1..] {
					if initialized_fields.iter().any(|i| arg.is_ident(i)) {
						args = quote! { #args, &(*this).#arg };
					} else if saved_positions.iter().any(|i| arg.is_ident(i)) {
						args = quote! { #args, #arg };
					} else {
						return Err("arguments to `delegate` after the first must be preceding fields or saved positions".to_string());
					}
				}
				(quote! { #func }, args)
			},
		},
	};
	Ok(quote! { #func(reader, #ptr #args)?; })
}

fn get_field_init(field: Field, initialized_fields: &[Ident], saved_positions: &mut Vec<Ident>) -> Result<TokenStream, String> {
	let FieldAttrs { boxed, zlib, delegate, list, save_pos, seek } = parse_field_attrs(field.attrs)?;
	let field_ident = field.ident.unwrap();
	let mut field_init = if let Some(len_arg) = list {
		if boxed {
			return Err("`list` field cannot also be `boxed`".to_string());
		}
		let get_len = if matches!(len_arg.to_string().as_str(), "u8" | "u16" | "u32" | "u64") {
			quote! {
				let len = tr_readable::read_get::<_, #len_arg>(reader)? as usize;
			}
		} else if initialized_fields.contains(&len_arg) {
			quote! {
				let len = tr_readable::ToLen::get_len(&(*this).#len_arg);
			}
		} else {
			return Err("`list` argument must either be an unsigned integer type or a preceding field".to_string());
		};
		let slice_init = match delegate {
			None => quote! {
				tr_readable::read_into_slice(reader, slice.as_mut_ptr(), len)?;
			},
			Some(delegate_args) => {
				let delegate_init = get_delegate_init(delegate_args, quote! { item.as_mut_ptr() }, initialized_fields, saved_positions)?;
				quote! {
					for item in &mut slice {
						#delegate_init
					}
				}
			},
		};
		quote! {
			{
				#get_len
				let mut slice = Box::new_uninit_slice(len);
				#slice_init
				(&raw mut (*this).#field_ident).write(slice.assume_init());
			}
		}
	} else if let Some(delegate_args) = delegate {
		get_delegate_init(delegate_args, quote! { &raw mut (*this).#field_ident }, initialized_fields, saved_positions)?
	} else if boxed {
		quote! {
			{
				let mut boxed = Box::new_uninit();
				tr_readable::read_into(reader, boxed.as_mut_ptr())?;
				(&raw mut (*this).#field_ident).write(boxed.assume_init());
			}
		}
	} else {
		quote! { tr_readable::read_into(reader, &raw mut (*this).#field_ident)?; }
	};
	if zlib {
		field_init = quote! {
			{
				let reader = &mut tr_readable::zlib(reader)?;
				#field_init
			}
		};
	}
	let mut seek_tokens = quote! {};
	if let Some(pos_ident) = save_pos {
		seek_tokens = quote! {
			#seek_tokens
			let #pos_ident = reader.stream_position()?;
		};
		saved_positions.push(pos_ident);
	}
	if let Some(seek_args) = seek {
		let [seek_start, seek_arg] = &seek_args[..] else {
			return Err("`seek` must be given two arguments".to_string());
		};
		if !saved_positions.contains(seek_start) {
			return Err("the first argument to `seek` must be previously declared with `seek_start`".to_string());
		}
		if !initialized_fields.contains(seek_arg) {
			return Err("the second argument to `seek` must be a preceding field".to_string());
		}
		// println!("seeking: {} to {}", reader.stream_position()?, #seek_start + (*this).#seek_arg as u64);
		seek_tokens = quote! {
			#seek_tokens
			reader.seek(std::io::SeekFrom::Start(#seek_start + (*this).#seek_arg as u64))?;
		};
	}
	field_init = quote! {
		#seek_tokens
		#field_init
	};
	Ok(field_init)
}

pub fn derive_readable_impl(input: DeriveInput) -> TokenStream {
	let type_name = input.ident;
	let fields = match input.data {
		Data::Struct(DataStruct { fields: Fields::Named(FieldsNamed { named, .. }), .. }) => named,
		_ => panic!("only structs with named fields supported"),
	};
	let mut body = quote! {};
	let mut initialized_fields = vec![];
	let mut seeks_starts = vec![];
	for field in fields {
		let field_ident = field.ident.clone().unwrap();//safe to unwrap, named fields only
		let field_init = match get_field_init(field, &initialized_fields, &mut seeks_starts) {
			Ok(init) => init,
			Err(e) => panic!("{}: {}", field_ident, e),
		};
		initialized_fields.push(field_ident);
		body = quote! {
			#body
			#field_init
		};
	}
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	quote! {
		impl #impl_generics tr_readable::Readable for #type_name #ty_generics #where_clause {
			unsafe fn read<R: std::io::BufRead + std::io::Seek>(reader: &mut R, this: *mut Self) -> std::io::Result<()> {
				#body
				Ok(())
			}
		}
	}
}
