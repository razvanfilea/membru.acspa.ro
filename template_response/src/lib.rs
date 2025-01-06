use quote::quote;
use synstructure::{AddBounds, Structure};

fn derive_template_response(mut structure: Structure<'_>) -> proc_macro2::TokenStream {
    structure
        .underscore_const(true)
        .add_bounds(AddBounds::None)
        .gen_impl(quote! {
            gen impl axum::response::IntoResponse for @Self {
                fn into_response(self) -> axum::response::Response {
                    let mime = Self::MIME_TYPE;
                    let try_into_response = |template: Self| -> Result<axum::response::Response, askama::Error> {
                        let value = template.render()?.into();
                        axum::response::Response::builder()
                            .header(
                                axum::http::header::CONTENT_TYPE,
                                axum::http::header::HeaderValue::from_static(mime),
                            )
                            .body(value)
                            .map_err(|err| askama::Error::Custom(err.into()))
                    };

                    try_into_response(self)
                        .map_err(|err| axum::response::ErrorResponse::from(err.to_string()))
                        .into_response()
                }
            }
        })
}

synstructure::decl_derive!([TemplateResponse] => derive_template_response);