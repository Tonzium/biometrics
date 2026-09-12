mod auth;
mod data;
mod health;
mod polar;
mod sync;

use axum::Router;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Polar Data Hub API",
        description = "Omat Polar Flow -tiedot: harjoitukset, uni, palautuminen ja aktiivisuus. \
                       Lukureitit ovat julkisia (PUBLIC_READ), omistajan reitit vaativat istuntocookien.",
        license(name = "MIT")
    ),
    tags(
        (name = "system", description = "Tila ja asetukset"),
        (name = "auth", description = "Kirjautuminen"),
        (name = "polar", description = "Polar-tilin yhdistäminen (omistaja)"),
        (name = "sync", description = "Synkronointi (omistaja)"),
        (name = "data", description = "Polar-data"),
        (name = "summary", description = "Yhteenvedot dashboardille")
    )
)]
struct ApiDoc;

/// Koko sovelluksen reititin. Kaikki reitit ovat `/api`-etuliitteen alla,
/// jotta nginx voi ohjata ne yksiselitteisesti backendille.
/// OpenAPI-kuvaus on osoitteessa `/api/openapi.json` ja Swagger UI `/api/docs`.
pub fn router(state: AppState) -> Router {
    let api = OpenApiRouter::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(polar::router())
        .merge(sync::router())
        .merge(data::records::router())
        .merge(data::summary::router());

    let (router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", api)
        .split_for_parts();

    router
        // validator_url("none") estää Swagger UI:n online-validator-merkin: se
        // latautuisi validator.swagger.io:sta, vuotaisi API-kuvauksen osoitteen
        // kolmannelle osapuolelle ja osuisi nyt myös CSP:n img-src-rajaan.
        .merge(
            SwaggerUi::new("/api/docs")
                .url("/api/openapi.json", openapi)
                .config(Config::default().validator_url("none")),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
