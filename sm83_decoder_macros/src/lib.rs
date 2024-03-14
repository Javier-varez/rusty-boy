use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use std::collections::hash_map::HashMap;
use syn::{braced, bracketed, parse::Parse, parse_macro_input, punctuated::Punctuated, Token};

struct VarDecl {
    var_ident: syn::Ident,
    ty: syn::Ident,
}

impl Parse for VarDecl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let var_ident: syn::Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let ty = input.parse()?;

        let var_ident_str = var_ident.to_string();
        if var_ident_str.len() > 1 {
            return Err(syn::Error::new(
                var_ident.span(),
                "Identifier must be a single letter",
            ));
        }

        Ok(Self { var_ident, ty })
    }
}

struct DecodingStatement {
    var_decls: Punctuated<VarDecl, Token![,]>,
    bit_pattern: syn::LitStr,
    body: TokenStream,
}

impl Parse for DecodingStatement {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let var_decls = content.parse_terminated(VarDecl::parse, Token![,])?;

        let mut known_vars = vec![];
        for var_decl in &var_decls {
            if known_vars
                .iter()
                .find(|v| **v == var_decl.var_ident)
                .is_some()
            {
                return Err(syn::Error::new(
                    var_decl.var_ident.span(),
                    "Variable redefined",
                ));
            }
            known_vars.push(var_decl.var_ident.clone());
        }

        let bit_pattern: syn::LitStr = input.parse()?;
        let _arrow: Token![=>] = input.parse()?;
        let content;
        let _ = braced!(content in input);
        let body: TokenStream = content.parse()?;

        Ok(Self {
            var_decls,
            bit_pattern,
            body,
        })
    }
}

struct Field<'a> {
    msb: usize,
    lsb: usize,
    ty: &'a Declaration,
    var_name: char,
}

impl<'a> Field<'a> {
    fn value_mask(&self) -> usize {
        return 1 << (self.msb - self.lsb + 1);
    }
}

impl DecodingStatement {
    fn generate_field_from_pattern<'a>(
        bit_pattern: &syn::LitStr,
        var_decl: &VarDecl,
        type_decls: &'a Declarations,
    ) -> syn::Result<Field<'a>> {
        let bit_pattern_str = bit_pattern.value();
        let expected_char = var_decl.var_ident.to_string().chars().next().unwrap();

        let msb = bit_pattern_str
            .chars()
            .into_iter()
            .enumerate()
            .find(|(_, c)| *c == expected_char)
            .map(|(idx, _)| bit_pattern_str.len() - idx - 1);

        if msb.is_none() {
            let message = format!("Bit pattern does not use variable {}", expected_char);
            return Err(syn::Error::new(bit_pattern.span(), message));
        }

        let lsb = bit_pattern_str
            .chars()
            .rev()
            .into_iter()
            .enumerate()
            .find(|(_, c)| *c == expected_char)
            .map(|(idx, _)| idx);

        // At this point we know that both are valid (if msb is found, lsb must also be found),
        // so unwrapping is fine
        let lsb = lsb.unwrap();
        let msb = msb.unwrap();

        let is_range_contiguous = bit_pattern_str
            .chars()
            .rev()
            .into_iter()
            .skip(lsb)
            .take(msb - lsb + 1)
            .fold(true, |valid, current| valid && (current == expected_char));

        if !is_range_contiguous {
            let message = format!("Bit pattern is not contiguous! `{}`", expected_char);
            return Err(syn::Error::new(bit_pattern.span(), message));
        }

        let ty = type_decls.find_type(&var_decl.ty);
        if ty.is_none() {
            let message = format!("Unknown type `{}`", var_decl.ty);
            return Err(syn::Error::new(var_decl.ty.span(), message));
        }
        let ty = ty.unwrap();

        let field = Field {
            msb,
            lsb,
            ty,
            var_name: expected_char,
        };

        for decl_val in &ty.values {
            let parsed_val: usize = decl_val.value.base10_parse()?;
            if (parsed_val & field.value_mask()) != 0 {
                let message = format!(
                    "Cannot fit value {} of variable {}",
                    parsed_val, var_decl.var_ident
                );
                return Err(syn::Error::new(bit_pattern.span(), message));
            }
        }

        Ok(field)
    }

    fn map_tokens(
        &self,
        stream: TokenStream,
        replacements: &HashMap<char, TokenStream>,
    ) -> syn::Result<TokenStream> {
        let mut result = TokenStream::new();
        let mut iter = stream.into_iter();
        while let Some(token) = iter.next() {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == '#' => {
                    let replacement_name = iter
                        .next()
                        .and_then(|v| match v {
                            TokenTree::Ident(id) if id.to_string().len() == 1 => {
                                Some(id.to_string())
                            }
                            _ => None,
                        })
                        .ok_or_else(|| {
                            syn::Error::new(
                                punct.span(),
                                "# is not followed by a valid variable identifier",
                            )
                        })?;

                    let Some(replacement) =
                        replacements.get(&replacement_name.chars().next().unwrap())
                    else {
                        return Err(syn::Error::new(punct.span(), "No replacement for variable"));
                    };
                    result.extend((*replacement).clone());
                }
                TokenTree::Group(group) => {
                    let inner = self.map_tokens(group.stream(), replacements)?;
                    let delimiter = group.delimiter();
                    let stream = match delimiter {
                        proc_macro2::Delimiter::Brace => quote::quote! {
                            { #inner }
                        },
                        proc_macro2::Delimiter::Parenthesis => quote::quote! {
                            ( #inner )
                        },
                        proc_macro2::Delimiter::Bracket => quote::quote! {
                            [ #inner ]
                        },
                        proc_macro2::Delimiter::None => quote::quote! {
                             #inner
                        },
                    };
                    result.extend(stream);
                }
                _ => result.extend(token.into_token_stream()),
            }
        }

        Ok(result)
    }

    fn replace_vars_in_body(
        &self,
        var_indexes: &[usize],
        fields: &[Field],
    ) -> syn::Result<TokenStream> {
        let mut replacements = HashMap::new();
        for (idx, field) in var_indexes.iter().zip(fields.iter()) {
            let type_ident = &field.ty.ident;
            let member_ident = &field.ty.values[*idx].label;
            let var_name = &field.var_name;

            let stream = quote::quote! {
                #type_ident::#member_ident
            };

            replacements.insert(*var_name, stream);
        }

        self.map_tokens(self.body.clone(), &replacements)
    }

    fn generate_map_entries(
        &self,
        types: &Declarations,
    ) -> syn::Result<HashMap<usize, DecoderRow>> {
        let mut fields = vec![];
        let mut var_indexes = vec![];

        // Reverses the pattern to start on lsb
        let mut zeroed_pattern: String = self.bit_pattern.value().chars().rev().collect();

        // We need to take the variables and create all combinatorics for each variable.
        for var_decl in &self.var_decls {
            let field = Self::generate_field_from_pattern(&self.bit_pattern, var_decl, types)?;

            zeroed_pattern = zeroed_pattern
                .chars()
                .into_iter()
                .enumerate()
                .map(|(idx, c)| {
                    if idx >= field.lsb && idx <= field.msb {
                        '0'
                    } else {
                        c
                    }
                })
                .collect();

            fields.push(field);
            var_indexes.push(0_usize);
        }

        // Reverses the pattern to start on msb and parses it.
        let zeroed_pattern: String = zeroed_pattern.chars().rev().collect();
        let zeroed_pattern = usize::from_str_radix(&zeroed_pattern, 2).map_err(|_| {
            syn::Error::new(self.bit_pattern.span(), "Pattern contains unexpanded data")
        })?;

        let dims = fields.len();

        let mut map = HashMap::new();
        'outer: loop {
            let mut opcode = zeroed_pattern;
            for (field, index) in fields.iter().zip(var_indexes.iter()) {
                let field_val: usize = field
                    .ty
                    .values
                    .iter()
                    .skip(*index)
                    .next()
                    .expect("Invalid field index! This is a bug in the macro!")
                    .value
                    .base10_parse()?;

                opcode |= field_val << field.lsb;
            }

            map.insert(
                opcode,
                DecoderRow {
                    token_stream: self.replace_vars_in_body(&var_indexes, &fields)?,
                },
            );

            // Trigger next permutation or exit
            let mut current_var = 0;
            'inner: loop {
                if current_var >= dims {
                    break 'outer;
                }

                let next_idx = var_indexes[current_var] + 1;
                let field = &fields[current_var];
                if next_idx == field.ty.values.len() {
                    var_indexes[current_var] = 0;
                    current_var += 1;
                } else {
                    var_indexes[current_var] = next_idx;
                    break 'inner;
                }
            }
        }
        Ok(map)
    }
}

struct DeclValue {
    label: syn::Ident,
    value: syn::LitInt,
}

impl Parse for DeclValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let label: syn::Ident = input.parse()?;
        let _: Token![=] = input.parse()?;
        let value: syn::LitInt = input.parse()?;
        Ok(Self { label, value })
    }
}

struct Declaration {
    ident: syn::Ident,
    values: Punctuated<DeclValue, Token![,]>,
}

impl Parse for Declaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let content;
        let _ = braced!(content in input);
        let values = content.parse_terminated(DeclValue::parse, Token![,])?;

        let mut known_labels: Vec<String> = vec![];
        let mut known_values: Vec<usize> = vec![];

        for decl_val in values.iter() {
            let label = decl_val.label.to_string();
            if known_labels.iter().find(|v| *v == &label).is_some() {
                return Err(syn::Error::new(decl_val.label.span(), "Duplicated tag"));
            } else {
                known_labels.push(label);
            }

            let value: usize = decl_val.value.base10_parse()?;
            if known_values.iter().find(|v| **v == value).is_some() {
                return Err(syn::Error::new(decl_val.value.span(), "Duplicated value"));
            } else {
                known_values.push(value);
            }
        }

        Ok(Self { ident, values })
    }
}

struct Declarations {
    decls: Punctuated<Declaration, Token![,]>,
}

impl Parse for Declarations {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        if ident != "Declarations" {
            return Err(syn::Error::new(
                ident.span(),
                "Expected `Declarations` identifier",
            ));
        }

        let content;
        let _braces = braced!(content in input);

        let decls = content.parse_terminated(Declaration::parse, Token![,])?;

        let mut known_names: Vec<syn::Ident> = vec![];
        for decl in &decls {
            if known_names
                .iter()
                .find(|n: &&syn::Ident| **n == decl.ident)
                .is_some()
            {
                let err = syn::Error::new(decl.ident.span(), "Duplicated declaration");
                return Err(err);
            }
            known_names.push(decl.ident.clone());
        }

        Ok(Self { decls })
    }
}

impl Declarations {
    fn find_type(&self, ty: &syn::Ident) -> Option<&Declaration> {
        return self
            .decls
            .iter()
            .find(|d| d.ident.to_string() == ty.to_string());
    }
}

struct DecoderRow {
    token_stream: TokenStream,
}

struct DecoderTable {
    table_ident: syn::Ident,
    element_type: syn::Type,
    table_size: usize,
    statements: Punctuated<DecodingStatement, Token![,]>,
}

impl Parse for DecoderTable {
    /// Parses a SeqItem from a ParseStream
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let table_ident: syn::Ident = input.parse()?;
        let _: Token![:] = input.parse()?;

        let table_args;
        let _brackets = bracketed!(table_args in input);
        let element_type = table_args.parse()?;
        let _: Token![;] = table_args.parse()?;
        let table_size = table_args.parse::<syn::LitInt>()?.base10_parse()?;

        let table_statements;
        let _braces = braced!(table_statements in input);

        let statements = table_statements.parse_terminated(DecodingStatement::parse, Token![,])?;
        Ok(Self {
            table_ident,
            element_type,
            table_size,
            statements,
        })
    }
}

impl DecoderTable {
    fn generate(&self, declarations: &Declarations) -> TokenStream {
        let table_name = &self.table_ident;
        let element_type = &self.element_type;
        let table_size = &self.table_size;

        let mut hash_table = HashMap::new();
        for statement in &self.statements {
            match statement.generate_map_entries(declarations) {
                Err(err) => return err.to_compile_error(),
                Ok(map_entries) => {
                    hash_table.extend(map_entries.into_iter());
                }
            }
        }

        let mut entries = vec![];
        for i in 0..self.table_size {
            if let Some(entry) = hash_table.get(&i) {
                let stream = &entry.token_stream;
                entries.push(quote::quote! { #stream });
            } else {
                let err = syn::Error::new(table_name.span(), format!("No match for opcode {i:#x}"))
                    .to_compile_error();
                entries.push(err);
            }
        }

        quote::quote! {
            pub const #table_name : [#element_type; #table_size] = [
                #(#entries),*
            ];
        }
    }
}

struct DecoderTables {
    decls: Declarations,
    tables: Punctuated<DecoderTable, Token![,]>,
}

impl Parse for DecoderTables {
    /// Parses a SeqItem from a ParseStream
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let decls: Declarations = input.parse()?;

        let tables: Punctuated<DecoderTable, Token![,]> =
            input.parse_terminated(DecoderTable::parse, Token![,])?;
        Ok(Self { decls, tables })
    }
}

impl DecoderTables {
    fn generate(&self) -> TokenStream {
        let enum_decls = self.generate_enum_decls();
        let table_decls = self.tables.iter().map(|t| t.generate(&self.decls));
        quote::quote! {
            #enum_decls
            #(#table_decls)*
        }
        .into()
    }

    fn generate_enum_decls(&self) -> TokenStream {
        let mut stream = TokenStream::new();
        for enum_decl in &self.decls.decls {
            let name = &enum_decl.ident;
            let val_decls = enum_decl.values.iter().map(|v| {
                let field_name = &v.label;
                let field_value = &v.value;
                quote::quote! {
                    #field_name = #field_value
                }
            });

            let enum_tokens = quote::quote! {
                pub enum #name {
                    #(#val_decls),*
                }
            };
            stream.extend(enum_tokens);
        }
        stream.into()
    }
}

#[proc_macro]
pub fn generate_decoder_tables(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let table = parse_macro_input!(input as DecoderTables);
    table.generate().into()
}
