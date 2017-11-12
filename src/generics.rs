use super::*;
use delimited::Delimited;

ast_struct! {
    /// Represents lifetimes and type parameters attached to a declaration
    /// of a function, enum, trait, etc.
    #[derive(Default)]
    pub struct Generics {
        pub lt_token: Option<Token![<]>,
        pub gt_token: Option<Token![>]>,
        pub lifetimes: Delimited<LifetimeDef, Token![,]>,
        pub ty_params: Delimited<TypeParam, Token![,]>,
        pub where_clause: WhereClause,
    }
}

#[cfg(feature = "printing")]
#[cfg_attr(feature = "extra-traits", derive(Debug, Eq, PartialEq, Hash))]
#[cfg_attr(feature = "clone-impls", derive(Clone))]
/// Returned by `Generics::split_for_impl`.
pub struct ImplGenerics<'a>(&'a Generics);

#[cfg(feature = "printing")]
#[cfg_attr(feature = "extra-traits", derive(Debug, Eq, PartialEq, Hash))]
#[cfg_attr(feature = "clone-impls", derive(Clone))]
/// Returned by `Generics::split_for_impl`.
pub struct TypeGenerics<'a>(&'a Generics);

#[cfg(feature = "printing")]
#[cfg_attr(feature = "extra-traits", derive(Debug, Eq, PartialEq, Hash))]
#[cfg_attr(feature = "clone-impls", derive(Clone))]
/// Returned by `TypeGenerics::as_turbofish`.
pub struct Turbofish<'a>(&'a Generics);

#[cfg(feature = "printing")]
impl Generics {
    /// Split a type's generics into the pieces required for impl'ing a trait
    /// for that type.
    ///
    /// ```
    /// # extern crate syn;
    /// # #[macro_use]
    /// # extern crate quote;
    /// # fn main() {
    /// # let generics: syn::Generics = Default::default();
    /// # let name = syn::Ident::from("MyType");
    /// let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    /// quote! {
    ///     impl #impl_generics MyTrait for #name #ty_generics #where_clause {
    ///         // ...
    ///     }
    /// }
    /// # ;
    /// # }
    /// ```
    pub fn split_for_impl(&self) -> (ImplGenerics, TypeGenerics, &WhereClause) {
        (ImplGenerics(self), TypeGenerics(self), &self.where_clause)
    }
}

#[cfg(feature = "printing")]
impl<'a> TypeGenerics<'a> {
    /// Turn a type's generics like `<X, Y>` into a turbofish like `::<X, Y>`.
    pub fn as_turbofish(&self) -> Turbofish {
        Turbofish(self.0)
    }
}

ast_struct! {
    /// A set of bound lifetimes, e.g. `for<'a, 'b, 'c>`
    #[derive(Default)]
    pub struct BoundLifetimes {
        pub for_token: Token![for],
        pub lt_token: Token![<],
        pub lifetimes: Delimited<LifetimeDef, Token![,]>,
        pub gt_token: Token![>],
    }
}

ast_struct! {
    /// A lifetime definition, e.g. `'a: 'b+'c+'d`
    pub struct LifetimeDef {
        pub attrs: Vec<Attribute>,
        pub lifetime: Lifetime,
        pub colon_token: Option<Token![:]>,
        pub bounds: Delimited<Lifetime, Token![+]>,
    }
}

impl LifetimeDef {
    pub fn new(lifetime: Lifetime) -> Self {
        LifetimeDef {
            attrs: Vec::new(),
            lifetime: lifetime,
            colon_token: None,
            bounds: Delimited::new(),
        }
    }
}

ast_struct! {
    /// A generic type parameter, e.g. `T: Into<String>`.
    pub struct TypeParam {
        pub attrs: Vec<Attribute>,
        pub ident: Ident,
        pub colon_token: Option<Token![:]>,
        pub bounds: Delimited<TypeParamBound, Token![+]>,
        pub eq_token: Option<Token![=]>,
        pub default: Option<Type>,
    }
}

impl From<Ident> for TypeParam {
    fn from(ident: Ident) -> Self {
        TypeParam {
            attrs: vec![],
            ident: ident,
            colon_token: None,
            bounds: Delimited::new(),
            eq_token: None,
            default: None,
        }
    }
}

ast_enum! {
    /// The AST represents all type param bounds as types.
    /// `typeck::collect::compute_bounds` matches these against
    /// the "special" built-in traits (see `middle::lang_items`) and
    /// detects Copy, Send and Sync.
    pub enum TypeParamBound {
        Trait(PolyTraitRef, TraitBoundModifier),
        Region(Lifetime),
    }
}

ast_enum! {
    /// A modifier on a bound, currently this is only used for `?Sized`, where the
    /// modifier is `Maybe`. Negative bounds should also be handled here.
    #[cfg_attr(feature = "clone-impls", derive(Copy))]
    pub enum TraitBoundModifier {
        None,
        Maybe(Token![?]),
    }
}

ast_struct! {
    /// A `where` clause in a definition
    #[derive(Default)]
    pub struct WhereClause {
        pub where_token: Option<Token![where]>,
        pub predicates: Delimited<WherePredicate, Token![,]>,
    }
}

impl WhereClause {
    pub fn none() -> Self {
        WhereClause::default()
    }
}

ast_enum_of_structs! {
    /// A single predicate in a `where` clause
    pub enum WherePredicate {
        /// A type binding, e.g. `for<'c> Foo: Send+Clone+'c`
        pub BoundPredicate(WhereBoundPredicate {
            /// Any lifetimes from a `for` binding
            pub bound_lifetimes: Option<BoundLifetimes>,
            /// The type being bounded
            pub bounded_ty: Type,
            pub colon_token: Token![:],
            /// Trait and lifetime bounds (`Clone+Send+'static`)
            pub bounds: Delimited<TypeParamBound, Token![+]>,
        }),

        /// A lifetime predicate, e.g. `'a: 'b+'c`
        pub RegionPredicate(WhereRegionPredicate {
            pub lifetime: Lifetime,
            pub colon_token: Option<Token![:]>,
            pub bounds: Delimited<Lifetime, Token![+]>,
        }),

        /// An equality predicate (unsupported)
        pub EqPredicate(WhereEqPredicate {
            pub lhs_ty: Type,
            pub eq_token: Token![=],
            pub rhs_ty: Type,
        }),
    }
}

#[cfg(feature = "parsing")]
pub mod parsing {
    use super::*;

    use synom::Synom;

    impl Synom for Generics {
        named!(parse -> Self, map!(
            alt!(
                do_parse!(
                    lt: punct!(<) >>
                    lifetimes: call!(Delimited::parse_terminated) >>
                    ty_params: cond!(
                        lifetimes.is_empty() || lifetimes.trailing_delim(),
                        call!(Delimited::parse_terminated)
                    ) >>
                    gt: punct!(>) >>
                    (lifetimes, ty_params, Some(lt), Some(gt))
                )
                |
                epsilon!() => { |_| (Delimited::new(), None, None, None) }
            ),
            |(lifetimes, ty_params, lt, gt)| Generics {
                lifetimes: lifetimes,
                ty_params: ty_params.unwrap_or_default(),
                where_clause: WhereClause::default(),
                gt_token: gt,
                lt_token: lt,
            }
        ));
    }

    impl Synom for LifetimeDef {
        named!(parse -> Self, do_parse!(
            attrs: many0!(call!(Attribute::parse_outer)) >>
            life: syn!(Lifetime) >>
            colon: option!(punct!(:)) >>
            bounds: cond!(
                colon.is_some(),
                call!(Delimited::parse_separated_nonempty)
            ) >>
            (LifetimeDef {
                attrs: attrs,
                lifetime: life,
                bounds: bounds.unwrap_or_default(),
                colon_token: colon.map(|_| <Token![:]>::default()),
            })
        ));
    }

    impl Synom for BoundLifetimes {
        named!(parse -> Self, do_parse!(
            for_: keyword!(for) >>
            lt: punct!(<) >>
            lifetimes: call!(Delimited::parse_terminated) >>
            gt: punct!(>) >>
            (BoundLifetimes {
                for_token: for_,
                lt_token: lt,
                gt_token: gt,
                lifetimes: lifetimes,
            })
        ));
    }

    impl Synom for TypeParam {
        named!(parse -> Self, do_parse!(
            attrs: many0!(call!(Attribute::parse_outer)) >>
            id: syn!(Ident) >>
            colon: option!(punct!(:)) >>
            bounds: cond!(
                colon.is_some(),
                call!(Delimited::parse_separated_nonempty)
            ) >>
            default: option!(do_parse!(
                eq: punct!(=) >>
                ty: syn!(Type) >>
                (eq, ty)
            )) >>
            (TypeParam {
                attrs: attrs,
                ident: id,
                bounds: bounds.unwrap_or_default(),
                colon_token: colon,
                eq_token: default.as_ref().map(|d| Token![=]((d.0).0)),
                default: default.map(|d| d.1),
            })
        ));
    }

    impl Synom for TypeParamBound {
        named!(parse -> Self, alt!(
            do_parse!(
                question: punct!(?) >>
                poly: syn!(PolyTraitRef) >>
                (TypeParamBound::Trait(poly, TraitBoundModifier::Maybe(question)))
            )
            |
            syn!(Lifetime) => { TypeParamBound::Region }
            |
            syn!(PolyTraitRef) => {
                |poly| TypeParamBound::Trait(poly, TraitBoundModifier::None)
            }
        ));

        fn description() -> Option<&'static str> {
            Some("type parameter buond")
        }
    }

    impl Synom for WhereClause {
        named!(parse -> Self, alt!(
            do_parse!(
                where_: keyword!(where) >>
                predicates: call!(Delimited::parse_terminated) >>
                (WhereClause {
                    predicates: predicates,
                    where_token: Some(where_),
                })
            )
            |
            epsilon!() => { |_| WhereClause::default() }
        ));

        fn description() -> Option<&'static str> {
            Some("where clause")
        }
    }

    impl Synom for WherePredicate {
        named!(parse -> Self, alt!(
            do_parse!(
                ident: syn!(Lifetime) >>
                colon: option!(punct!(:)) >>
                bounds: cond!(
                    colon.is_some(),
                    call!(Delimited::parse_separated)
                ) >>
                (WherePredicate::RegionPredicate(WhereRegionPredicate {
                    lifetime: ident,
                    bounds: bounds.unwrap_or_default(),
                    colon_token: colon,
                }))
            )
            |
            do_parse!(
                bound_lifetimes: option!(syn!(BoundLifetimes)) >>
                bounded_ty: syn!(Type) >>
                colon: punct!(:) >>
                bounds: call!(Delimited::parse_separated_nonempty) >>
                (WherePredicate::BoundPredicate(WhereBoundPredicate {
                    bound_lifetimes: bound_lifetimes,
                    bounded_ty: bounded_ty,
                    bounds: bounds,
                    colon_token: colon,
                }))
            )
        ));
    }
}

#[cfg(feature = "printing")]
mod printing {
    use super::*;
    use attr::FilterAttrs;
    use quote::{Tokens, ToTokens};

    /// Returns true if the generics object has no lifetimes or ty_params.
    fn empty_normal_generics(generics: &Generics) -> bool {
        generics.lifetimes.is_empty() && generics.ty_params.is_empty()
    }

    /// We need a comma between the lifetimes list and the ty_params list if
    /// there are more than 0 lifetimes, the lifetimes list didn't have a
    /// trailing delimiter, and there are more than 0 type parameters. This is a
    /// helper method for adding that comma.
    fn maybe_add_lifetime_params_comma(tokens: &mut Tokens, generics: &Generics) {
        // We may need to require a trailing comma if we have any ty_params.
        if !generics.lifetimes.empty_or_trailing() && !generics.ty_params.is_empty() {
            <Token![,]>::default().to_tokens(tokens);
        }
    }

    impl ToTokens for Generics {
        fn to_tokens(&self, tokens: &mut Tokens) {
            if empty_normal_generics(self) {
                return;
            }

            TokensOrDefault(&self.lt_token).to_tokens(tokens);
            self.lifetimes.to_tokens(tokens);
            maybe_add_lifetime_params_comma(tokens, self);
            self.ty_params.to_tokens(tokens);
            TokensOrDefault(&self.gt_token).to_tokens(tokens);
        }
    }

    impl<'a> ToTokens for ImplGenerics<'a> {
        fn to_tokens(&self, tokens: &mut Tokens) {
            if empty_normal_generics(&self.0) {
                return;
            }

            TokensOrDefault(&self.0.lt_token).to_tokens(tokens);
            self.0.lifetimes.to_tokens(tokens);
            maybe_add_lifetime_params_comma(tokens, &self.0);
            for param in self.0.ty_params.iter() {
                 // Leave off the type parameter defaults
                let item = param.item();
                tokens.append_all(item.attrs.outer());
                item.ident.to_tokens(tokens);
                if !item.bounds.is_empty() {
                    TokensOrDefault(&item.colon_token).to_tokens(tokens);
                    item.bounds.to_tokens(tokens);
                }
                param.delimiter().to_tokens(tokens);
            }
            TokensOrDefault(&self.0.gt_token).to_tokens(tokens);
        }
    }

    impl<'a> ToTokens for TypeGenerics<'a> {
        fn to_tokens(&self, tokens: &mut Tokens) {
            if empty_normal_generics(&self.0) {
                return;
            }

            TokensOrDefault(&self.0.lt_token).to_tokens(tokens);
            // Leave off the lifetime bounds and attributes
            for param in self.0.lifetimes.iter() {
                param.item().lifetime.to_tokens(tokens);
                param.delimiter().to_tokens(tokens);
            }
            maybe_add_lifetime_params_comma(tokens, &self.0);
            // Leave off the type parameter defaults
            for param in self.0.ty_params.iter() {
                param.item().ident.to_tokens(tokens);
                param.delimiter().to_tokens(tokens);
            }
            TokensOrDefault(&self.0.gt_token).to_tokens(tokens);
        }
    }

    impl<'a> ToTokens for Turbofish<'a> {
        fn to_tokens(&self, tokens: &mut Tokens) {
            if !empty_normal_generics(&self.0) {
                <Token![::]>::default().to_tokens(tokens);
                TypeGenerics(self.0).to_tokens(tokens);
            }
        }
    }

    impl ToTokens for BoundLifetimes {
        fn to_tokens(&self, tokens: &mut Tokens) {
            self.for_token.to_tokens(tokens);
            self.lt_token.to_tokens(tokens);
            self.lifetimes.to_tokens(tokens);
            self.gt_token.to_tokens(tokens);
        }
    }

    impl ToTokens for LifetimeDef {
        fn to_tokens(&self, tokens: &mut Tokens) {
            tokens.append_all(self.attrs.outer());
            self.lifetime.to_tokens(tokens);
            if !self.bounds.is_empty() {
                TokensOrDefault(&self.colon_token).to_tokens(tokens);
                self.bounds.to_tokens(tokens);
            }
        }
    }

    impl ToTokens for TypeParam {
        fn to_tokens(&self, tokens: &mut Tokens) {
            tokens.append_all(self.attrs.outer());
            self.ident.to_tokens(tokens);
            if !self.bounds.is_empty() {
                TokensOrDefault(&self.colon_token).to_tokens(tokens);
                self.bounds.to_tokens(tokens);
            }
            if self.default.is_some() {
                TokensOrDefault(&self.eq_token).to_tokens(tokens);
                self.default.to_tokens(tokens);
            }
        }
    }

    impl ToTokens for TypeParamBound {
        fn to_tokens(&self, tokens: &mut Tokens) {
            match *self {
                TypeParamBound::Region(ref lifetime) => lifetime.to_tokens(tokens),
                TypeParamBound::Trait(ref trait_ref, ref modifier) => {
                    modifier.to_tokens(tokens);
                    trait_ref.to_tokens(tokens);
                }
            }
        }
    }

    impl ToTokens for TraitBoundModifier {
        fn to_tokens(&self, tokens: &mut Tokens) {
            match *self {
                TraitBoundModifier::None => {}
                TraitBoundModifier::Maybe(ref t) => t.to_tokens(tokens),
            }
        }
    }

    impl ToTokens for WhereClause {
        fn to_tokens(&self, tokens: &mut Tokens) {
            if !self.predicates.is_empty() {
                TokensOrDefault(&self.where_token).to_tokens(tokens);
                self.predicates.to_tokens(tokens);
            }
        }
    }

    impl ToTokens for WhereBoundPredicate {
        fn to_tokens(&self, tokens: &mut Tokens) {
            self.bound_lifetimes.to_tokens(tokens);
            self.bounded_ty.to_tokens(tokens);
            self.colon_token.to_tokens(tokens);
            self.bounds.to_tokens(tokens);
        }
    }

    impl ToTokens for WhereRegionPredicate {
        fn to_tokens(&self, tokens: &mut Tokens) {
            self.lifetime.to_tokens(tokens);
            if !self.bounds.is_empty() {
                TokensOrDefault(&self.colon_token).to_tokens(tokens);
                self.bounds.to_tokens(tokens);
            }
        }
    }

    impl ToTokens for WhereEqPredicate {
        fn to_tokens(&self, tokens: &mut Tokens) {
            self.lhs_ty.to_tokens(tokens);
            self.eq_token.to_tokens(tokens);
            self.rhs_ty.to_tokens(tokens);
        }
    }
}
