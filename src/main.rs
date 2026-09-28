use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use livekit_api::access_token::{AccessToken, VideoGrants};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// Konfiguration deiner LiveKit-Zugangsdaten (idealerweise aus Umgebungsvariablen laden)
const LIVEKIT_API_KEY: &str = "devkey";
const LIVEKIT_API_SECRET: &str = "secret";

// -----------------------------------------------------------------------------
// REQUEST-PARAMETER DEFINITION
// -----------------------------------------------------------------------------
// Diese Struktur repräsentiert die Parameter, die ein Client (z. B. Element Call oder
// deine Matrix-Integration) an diesen Auth-Server sendet, um ein Token zu verlangen.
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    /// Der Raumname in LiveKit, dem der Benutzer beitreten möchte.
    /// In Matrix entspricht dies meist einer eindeutigen Call-Room-ID / Call-ID.
    pub room: Option<String>,

    /// Die eindeutige Identität des Benutzers (User ID).
    /// Bei Matrix z. B. `@username:domain.com` oder eine anonymisierte Session-ID.
    pub identity: Option<String>,

    /// Optional: Der Name, der anderen Teilnehmern im Raum angezeigt werden soll.
    pub name: Option<String>,

    /// Optional: Steuert, ob der Benutzer direkt als Admin/Moderator beitritt.
    pub is_admin: Option<bool>,
}

// Struct für die JSON-Antwort an den Client
#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
}

#[tokio::main]
async fn main() {
    // Router aufsetzen: Akzeptiert GET und POST auf /token
    let app = Router::new()
        .route("/token", get(handle_token_get))
        .route("/token", post(handle_token_post));

    let addr = SocketAddr::from(([0, 0, 0, 0], 7880));
    println!("LiveKit Auth-Server läuft auf http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handler für HTTP GET (z. B. via Query-Parameter: /token?room=room1&identity=user1)
async fn handle_token_get(Query(req): Query<TokenRequest>) -> impl IntoResponse {
    generate_token_response(req)
}

// Handler für HTTP POST (z. B. via JSON Body)
async fn handle_token_post(Json(req): Json<TokenRequest>) -> impl IntoResponse {
    generate_token_response(req)
}

fn generate_token_response(req: TokenRequest) -> impl IntoResponse {
    // Standardwerte verwenden, falls der Client keine Parameter mitgeschickt hat
    let room_name = req.room.unwrap_or_else(|| "default-room".to_string());
    let user_identity = req.identity.unwrap_or_else(|| "anonymous-user".to_string());
    let display_name = req.name.unwrap_or_else(|| user_identity.clone());

    /* 
     ================================================================================
     HINWEIS ZUR AUTHENTIFIZIERUNG (ECHTE PRODUCTION-LOGIK):
     ================================================================================
     Aktuell wird hier jedes Token blind ausgestellt ("egal wie der Request aussieht").
     
     Um den Server sicher zu machen und ECHTE Authentifizierung zu erzwingen, 
     müsstest du an dieser Stelle Folgendes tun:

     1. Matrix OpenID / Access-Token Validierung:
        - Lies den HTTP `Authorization`-Header vom Client aus (z. B. `Bearer <matrix_access_token>`).
        - Oder fordere vom Client ein Matrix OpenID Token an.
        - Sende einen Request an deinen Matrix Synapse / Dendrite Server (`/_matrix/client/v3/account/whoami` 
          oder `/_matrix/federation/v1/openid/userinfo`), um zu prüfen, ob das Token gültig ist 
          und zu welchem `@user:domain.tld` es gehört.

     2. Berechtigungsprüfung (Matrix Room Membership):
        - Überprüfe via Matrix API, ob der verifizierte Benutzer `@user:domain.tld` überhaupt 
          Mitglied in dem Matrix-Raum ist, der dem LiveKit-`room` entspricht.
        - Falls NEIN: Breche hier ab und gib HTTP 401 Unauthorized oder HTTP 403 Forbidden zurück.

     3. Identität überschreiben:
        - Überschreibe `user_identity` mit der echten, von Matrix verifizierten User-ID und
          verwende NICHT unkritisch den vom Request übergebenen `identity`-Wert, da dieser sonst 
          gefälscht werden könnte (Identity Spoofing).
     ================================================================================
    */

    // Berechtigungen (Grants) für den LiveKit Room festlegen
    let mut grants = VideoGrants::default();
    grants.room_join = true;                     // Erlaubt das Beitreten zum Raum
    grants.room = room_name;                      // Zuweisung des Raumnamens
    grants.can_publish = Some(true);             // Erlaubt das Senden von Audio/Video
    grants.can_subscribe = Some(true);           // Erlaubt das Empfangen von Streams
    grants.can_publish_data = Some(true);        // Erlaubt Chat/Daten-Channels im Raum

    // Falls ein Admin-Flag mitgegeben wurde
    if req.is_admin.unwrap_or(false) {
        grants.room_admin = true;
    }

    // Erstellen und Signieren des LiveKit Access Tokens mit Key & Secret
    let token_result = AccessToken::with_api_key(LIVEKIT_API_KEY, LIVEKIT_API_SECRET)
        .with_identity(&user_identity)
        .with_name(&display_name)
        .with_grants(grants)
        .to_jwt();

    match token_result {
        Ok(jwt_string) => (StatusCode::OK, Json(TokenResponse { token: jwt_string })).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fehler beim Generieren des Tokens: {}", err),
        )
            .into_response(),
    }
}
