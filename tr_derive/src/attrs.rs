#![macro_use]

macro_rules! parse_attrs_field_type {
	(bool) => { bool };
	(Arg) => { Option<&'a proc_macro2::TokenStream> };
}

macro_rules! parse_attrs_field_default {
	(bool) => { false };
	(Arg) => { None };
}

macro_rules! parse_attrs_field_pattern {
	(bool, $tokens:ident) => { syn::Meta::Path(_) };
	(Arg, $tokens:ident) => { syn::Meta::List(syn::MetaList { $tokens, .. }) };
}

macro_rules! parse_attrs_field_value {
	(bool, $tokens:ident) => { true };
	(Arg, $tokens:ident) => { Some($tokens) };
}

macro_rules! parse_attrs_fn {
	(
		$fn_name:ident -> $type_name:ident {
			$($attr_name:ident: $attr_type:ident,)*
		}
	) => {
		struct $type_name<'a> {
			$($attr_name: parse_attrs_field_type!($attr_type),)*
		}
		
		fn $fn_name<'a>(attrs: &'a [syn::Attribute]) -> $type_name {
			$(let mut $attr_name = parse_attrs_field_default!($attr_type);)*
			for attr in attrs {
				if let Some(ident) = attr.path().get_ident() {
					match ident.to_string().as_str() {
						$(
							stringify!($attr_name) => match $attr_name {
								parse_attrs_field_default!($attr_type) => match &attr.meta {
									parse_attrs_field_pattern!($attr_type, tokens) => $attr_name = parse_attrs_field_value!($attr_type, tokens),
									_ => panic!("invalid use of helper attribute"),
								},
								_ => panic!("cannot use helper attribute more than once"),
							},
						)*
						_ => {},
					}
				}
			}
			$type_name { $($attr_name),* }
		}
	};
}
