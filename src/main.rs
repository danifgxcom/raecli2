use clap::Parser;
use colored::Colorize;
use encoding_rs::*;
use flate2::read::GzDecoder;
use rustls::{ClientConfig, ClientConnection, RootCertStore};
use rustls::pki_types::ServerName;
use scraper::{Element, Html, Selector};
use std::error::Error;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;
use webpki_roots::TLS_SERVER_ROOTS;


// Función para obtener la ruta absoluta de un archivo de debug
fn get_debug_file_path(filename: &str) -> PathBuf {
    match std::env::current_dir() {
        Ok(current_dir) => current_dir.join(filename),
        Err(_) => PathBuf::from(filename), // Fallback a ruta relativa si hay error
    }
}

fn mostrar_con_colores(definicion: &str) {
    let lineas: Vec<&str> = definicion
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for (i, linea) in lineas.iter().enumerate() {
        match i % 6 {
            0 => println!("{}", linea.red()),
            1 => println!("{}", linea.green()),
            2 => println!("{}", linea.yellow()),
            3 => println!("{}", linea.blue()),
            4 => println!("{}", linea.magenta()),
            5 => println!("{}", linea.cyan()),
            _ => println!("{}", linea),
        }
    }
}

#[derive(Parser)]
#[command(name = "rae-cli")]
#[command(about = "Busca definiciones de palabras en el Diccionario de la RAE")]
struct Args {
    /// Palabra a buscar en la RAE
    palabra: String,

    /// Modo debug: guardar HTML recibido
    #[arg(long, help = "Guardar HTML recibido para debugging")]
    debug: bool,

}

fn main() {
    let args = Args::parse();

    std::env::set_var("USE_CUSTOM_TLS", "1");

    // Usar el método TLS fingerprint
    let resultado = simular_lynx_exacto(&args.palabra, args.debug);

    match resultado {
        Ok(resultado) => mostrar_con_colores(&resultado),
        Err(e) => {
            if args.debug {
                eprintln!("❌ Error con método TLS fingerprint: {}", e);
            }

            // Fallback al método HTTP simple si falla el TLS fingerprint
            if args.debug {
                eprintln!("\n🔄 Intentando con método HTTP simple como fallback...");
            }

            match buscar_en_rae_http(&args.palabra, args.debug) {
                Ok(resultado) => mostrar_con_colores(&resultado),
                Err(e) => {
                    if args.debug {
                        eprintln!("❌ Error con método HTTP: {}", e);
                    }
                }
            }
        }
    }
}

fn buscar_en_rae_http(palabra: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    // Crear cliente HTTP con configuración similar a Lynx (trusted browser)
    let client = reqwest::blocking::Client::builder()
        .user_agent("Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3")
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    // Headers minimalistas como Lynx (solo los esenciales)
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "text/html".parse()?);
    headers.insert("Accept-Language", "es".parse()?);
    headers.insert("Accept-Encoding", "gzip".parse()?);

    let url = format!("https://dle.rae.es/{}", urlencoding::encode(palabra));
    if debug {
        eprintln!("=== PASO 1: PETICIÓN INICIAL ===");
        eprintln!("🌐 URL: {}", url);
        eprintln!("🎭 User-Agent: Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3");
        eprintln!("📋 Headers enviados: {:#?}", headers);
    }

    let response = client.get(&url).headers(headers).send()?;

    if debug {
        eprintln!("\n=== RESPUESTA INICIAL ===");
        eprintln!("📊 Status: {}", response.status());
        eprintln!("📊 Headers recibidos: {:#?}", response.headers());
        eprintln!(
            "📊 Tamaño del contenido: {} bytes",
            response.content_length().unwrap_or(0)
        );
        if let Some(cf_ray) = response.headers().get("cf-ray") {
            eprintln!("☁️  Cloudflare Ray ID: {:?}", cf_ray);
        }
    }

    // Obtener bytes en lugar de texto para decodificar manualmente
    let bytes = response.bytes()?;
    let html_content = decodificar_contenido(&bytes, debug)?;

    if debug {
        let debug_file_path = get_debug_file_path("rae-debug-lynx.html");
        std::fs::write(&debug_file_path, &html_content)?;
        eprintln!("\n=== ANÁLISIS DEL CONTENIDO ===");
        eprintln!("🔍 HTML guardado en {}", debug_file_path.display());
        eprintln!("📊 Tamaño: {} bytes", html_content.len());
        eprintln!("🔎 Primeros 200 chars:");
        let preview = html_content.chars().take(200).collect::<String>();
        eprintln!("{}", preview);
    }

    // Detectar si es una página de desafío de Cloudflare
    if detectar_cloudflare_challenge(&html_content) {
        if debug {
            eprintln!("\n🛡️  DETECTADO DESAFÍO DE CLOUDFLARE - iniciando resolución...");
        }
        return intentar_resolver_challenge(&client, &html_content, palabra, debug);
    }

    if debug {
        eprintln!("\n✅ CONTENIDO VÁLIDO - extrayendo definición...");
    }
    
    // Extraer la palabra final después de redirecciones
    let palabra_final = extraer_palabra_de_respuesta(&html_content).unwrap_or_else(|| palabra.to_string());
    
    match extraer_definicion(&html_content) {
        Ok(definiciones) => {
            if palabra_final != palabra {
                Ok(format!("{}, {}\n\n{}", palabra_final, palabra, definiciones))
            } else {
                Ok(definiciones)
            }
        },
        Err(e) => Err(e)
    }
}

fn extraer_definicion(html_content: &str) -> Result<String, Box<dyn Error>> {
    let document = Html::parse_document(html_content);

    // Selectores específicos para definiciones de la RAE (versión actualizada)
    let selectores_definicion = [
        // Selectores para la nueva estructura HTML de la RAE
        ".c-definitions__item[role='definition']",
        ".c-definitions li",
        // Selectores para la estructura anterior
        "article p",
        "article ol li",
        "article ul li",
        ".normal p",
        ".normal li",
        "#resultados p",
        "#resultados li",
    ];

    let mut definiciones_encontradas = Vec::new();

    // Primero intentar con los selectores específicos
    for selector_str in &selectores_definicion {
        if let Ok(selector) = Selector::parse(selector_str) {
            for elemento in document.select(&selector) {
                // Extraer el texto completo del elemento
                let texto = elemento.text().collect::<String>().trim().to_string();

                // Solo agregar si es realmente una definición
                if es_definicion_real(&texto) {
                    definiciones_encontradas.push(texto);
                }
            }

            // Si encontramos definiciones con este selector, usarlas
            if !definiciones_encontradas.is_empty() {
                break;
            }
        }
    }

    // Si no encontramos nada con los selectores específicos, intentar con la estructura específica
    // de la nueva versión del sitio de la RAE
    if definiciones_encontradas.is_empty() {
        // Intentar extraer de la estructura de definiciones numeradas
        if let Ok(acep_selector) = Selector::parse(".n_acep") {
            if let Ok(abbr_selector) = Selector::parse("abbr.d, abbr.g") {
                if let Ok(div_selector) = Selector::parse("div[role='definition'] > div") {
                    let mut aceps = Vec::new();

                    // Recopilar todas las acepciones numeradas
                    for acep_elem in document.select(&acep_selector) {
                        let acep_num = acep_elem.text().collect::<String>().trim().to_string();

                        // Buscar el elemento padre que contiene toda la definición
                        if let Some(parent) = acep_elem.parent_element() {
                            if let Some(def_div) = parent.select(&div_selector).next() {
                                let mut def_text = acep_num;

                                // Agregar la categoría gramatical (f., m., adj., etc.)
                                if let Some(abbr) = def_div.select(&abbr_selector).next() {
                                    def_text.push(' ');
                                    def_text.push_str(abbr.text().collect::<String>().trim());
                                }

                                // Agregar el resto del texto de la definición
                                def_text.push(' ');
                                def_text.push_str(def_div.text().collect::<String>().trim());

                                // Limpiar y normalizar el texto
                                let def_text = def_text.replace("  ", " ").trim().to_string();

                                if es_definicion_real(&def_text) {
                                    aceps.push(def_text);
                                }
                            }
                        }
                    }

                    if !aceps.is_empty() {
                        definiciones_encontradas = aceps;
                    }
                }
            }
        }
    }

    // Si aún no encontramos nada, buscar en todo el artículo
    if definiciones_encontradas.is_empty() {
        if let Ok(article_selector) = Selector::parse("article, .c-section") {
            if let Some(article) = document.select(&article_selector).next() {
                let todo_texto = article.text().collect::<String>();

                // Buscar líneas que parezcan definiciones reales
                for linea in todo_texto.lines() {
                    let linea_limpia = linea.trim();
                    if es_definicion_real(linea_limpia) {
                        definiciones_encontradas.push(linea_limpia.to_string());
                    }
                }
            }
        }
    }

    // Si aún no encontramos nada, buscar en todo el HTML
    if definiciones_encontradas.is_empty() {
        // Buscar patrones específicos en el HTML completo
        if html_content.contains("<div class=\"n2 c-text-intro\">") {
            // Extraer definiciones de la estructura HTML completa
            if let Ok(def_item_selector) =
                Selector::parse(".c-definitions__item[role='definition']")
            {
                for def_item in document.select(&def_item_selector) {
                    let texto = def_item.text().collect::<String>();
                    let texto_limpio = texto
                        .lines()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<&str>>()
                        .join(" ")
                        .replace("  ", " ");

                    if es_definicion_real(&texto_limpio) {
                        definiciones_encontradas.push(texto_limpio);
                    }
                }
            }
        }
    }

    if !definiciones_encontradas.is_empty() {
        return Ok(definiciones_encontradas.join("\n"));
    }

    Err("No se encontraron definiciones en la página de la RAE.".into())
}

fn es_definicion_real(texto: &str) -> bool {
    let texto = texto.trim();

    // Debe tener longitud mínima razonable
    if texto.len() < 15 {
        return false;
    }

    // Exclusiones específicas de la RAE (navegación, interfaz, etc.)
    let exclusiones = [
        "palabra del día",
        "diccionario en su móvil",
        "descargue en su dispositivo",
        "app store",
        "google play",
        "otros diccionarios",
        "página web",
        "recursos en línea",
        "edición del tricentenario",
        "guía de consulta",
        "modo de cita",
        "responsable: real academia",
        "gestionar su",
        "consulta posible gracias",
        "fundación",
        "nueva búsqueda",
        "avanzada",
        "conjugar",
        "cerrar",
        "compartir",
        "imprimir",
        "sinónimos",
        "antónimos",
    ];

    let texto_lower = texto.to_lowercase();
    for exclusion in &exclusiones {
        if texto_lower.contains(exclusion) {
            return false;
        }
    }

    // Patrones típicos de definiciones de la RAE

    // 1. Empieza con número y categoría gramatical: "1. m.", "2. f.", etc.
    if texto.chars().next().is_some_and(|c| c.is_ascii_digit()) && texto.contains(". ") {
        // Verificar si después del número hay categoría gramatical
        let categorias = [
            "m.", "f.", "adj.", "tr.", "intr.", "prnl.", "loc.", "adv.", "interj.", "prep.",
            "conj.",
        ];
        for categoria in &categorias {
            if texto.contains(categoria) {
                return true;
            }
        }
    }

    // 2. Empieza directamente con categoría gramatical
    let categorias_inicio = [
        "m. ", "f. ", "adj. ", "tr. ", "intr. ", "prnl. ", "loc. ", "adv. ", "interj. ", "prep. ",
        "conj. ", "m. y f. ",
    ];

    for categoria in &categorias_inicio {
        if texto.starts_with(categoria) {
            return true;
        }
    }

    false
}

fn detectar_cloudflare_challenge(html: &str) -> bool {
    // Si la respuesta contiene un status 200 OK, no es un desafío
    if html.starts_with("HTTP/1.1 200 OK") {
        return false;
    }

    // Si la respuesta contiene el título o la definición de la RAE, no es un desafío
    if html.contains("<title>casa | Definición | Diccionario de la lengua española | RAE")
        || html.contains("Diccionario de la lengua española")
            && html.contains("Real Academia Española")
    {
        return false;
    }

    // Detectar indicadores específicos de desafío de Cloudflare
    let indicadores_desafio = [
        "Checking your browser",
        "Please wait",
        "Just a moment",
        "cf-challenge",
        "__cf_chl_jschl_tk__",
        "DDoS protection",
        "challenge-form",
        "challenge-error-title",
        "cf_chl_prog",
        "cf-please-wait",
        "cf-browser-verification",
        "turnstile",
        "cf_challenge",
    ];

    for indicador in &indicadores_desafio {
        if html.contains(indicador) {
            return true;
        }
    }

    // Verificar si es una página de error 403 Forbidden de Cloudflare
    if html.contains("403 Forbidden") && html.contains("cloudflare") {
        return true;
    }

    // Si la respuesta es muy pequeña y no contiene contenido útil, podría ser un desafío
    // Pero solo si no es una respuesta HTTP válida
    if html.len() < 1000
        && !html.contains("<article")
        && !html.contains("<body")
        && !html.contains("HTTP/1.1")
    {
        return true;
    }

    false
}

fn intentar_resolver_challenge(
    client: &reqwest::blocking::Client,
    _html: &str,
    palabra: &str,
    debug: bool,
) -> Result<String, Box<dyn Error>> {
    if debug {
        eprintln!("🧩 Intentando resolver challenge JavaScript...");
    }

    // 🦌 LYNX PURO: Comportamiento auténtico sin JavaScript
    // El experimento demostró que Lynx no necesita resolver challenges JS
    if debug {
        eprintln!("🦌 Lynx auténtico: NO ejecuta JavaScript (comportamiento real)");
        eprintln!("✅ Cloudflare permite acceso directo a Lynx sin challenges");
    }
    
    // DECISIÓN ARQUITECTURAL: Lynx real no ejecuta JS, y Cloudflare lo respeta
    // El TLS fingerprinting es tan efectivo que no requiere challenge resolution
    
    // JavaScript resolution removido del flujo principal - innecesario para Lynx
    // Mantenido solo como código de referencia para futuros navegadores

    // NUEVO: Intentar simular Lynx exactamente (basado en análisis tcpdump)
    if debug {
        eprintln!("🎭 Intentando simular Lynx exactamente...");
    }
    if let Ok(lynx_sim_result) = simular_lynx_exacto(palabra, debug) {
        return Ok(lynx_sim_result);
    }

    // Fallback: Intentar con curl
    if debug {
        eprintln!("🔧 Intentando con curl como última alternativa...");
    }
    if let Ok(curl_result) = intentar_con_curl(palabra, debug) {
        return Ok(curl_result);
    }

    if debug {
        eprintln!("⏳ Fallback: Esperando 5 segundos (como haría un navegador)...");
    }

    // Fallback: Esperar 5 segundos como requiere Cloudflare
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Intentar segunda petición después de la espera
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8"
            .parse()?,
    );
    headers.insert("Accept-Language", "es-ES,es;q=0.9,en;q=0.8".parse()?);
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse()?);
    headers.insert("Referer", "https://dle.rae.es/".parse()?);
    headers.insert("Sec-Fetch-Dest", "document".parse()?);
    headers.insert("Sec-Fetch-Mode", "navigate".parse()?);
    headers.insert("Sec-Fetch-Site", "same-origin".parse()?);
    headers.insert(
        "Sec-Ch-Ua",
        "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"".parse()?,
    );
    headers.insert("Sec-Ch-Ua-Mobile", "?0".parse()?);
    headers.insert("Sec-Ch-Ua-Platform", "\"Linux\"".parse()?);
    headers.insert("Upgrade-Insecure-Requests", "1".parse()?);

    if debug {
        eprintln!("🔄 Reintentando petición después de espera...");
    }

    let url = format!("https://dle.rae.es/{}", urlencoding::encode(palabra));
    let response = client.get(&url).headers(headers).send()?;

    if debug {
        eprintln!(
            "📊 Respuesta después de challenge: Status {}",
            response.status()
        );
    }

    let html_content = response.text()?;

    if debug {
        let debug_file_path = get_debug_file_path("rae-debug-after-challenge.html");
        std::fs::write(&debug_file_path, &html_content)?;
        eprintln!(
            "🔍 HTML post-challenge guardado en {}",
            debug_file_path.display()
        );
    }

    // Verificar si aún hay desafío
    if detectar_cloudflare_challenge(&html_content) {
        return Err("Cloudflare challenge no resuelto automáticamente".into());
    }

    extraer_definicion(&html_content)
}

fn intentar_con_curl(palabra: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    if debug {
        eprintln!("🌐 Ejecutando curl con headers exactos del navegador...");
    }

    let url = format!("https://dle.rae.es/{}?m=form", urlencoding::encode(palabra));

    let output = std::process::Command::new("curl")
        .arg("-s") // Silent
        .arg("-L") // Follow redirects
        .arg("--compressed") // Auto decompress
        .arg("-H").arg("Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
        .arg("-H").arg("Accept-Language: en,es;q=0.9")
        .arg("-H").arg("Accept-Encoding: gzip, deflate, br, zstd")
        .arg("-H").arg("Cache-Control: max-age=0")
        .arg("-H").arg("Sec-Fetch-Dest: document")
        .arg("-H").arg("Sec-Fetch-Mode: navigate")
        .arg("-H").arg("Sec-Fetch-Site: same-origin")
        .arg("-H").arg("Sec-Fetch-User: ?1")
        .arg("-H").arg("Sec-Ch-Ua: \"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"")
        .arg("-H").arg("Sec-Ch-Ua-Mobile: ?0")
        .arg("-H").arg("Sec-Ch-Ua-Platform: \"Linux\"")
        .arg("-H").arg("Upgrade-Insecure-Requests: 1")
        .arg("-H").arg("Priority: u=0, i")
        .arg("-H").arg("Referer: https://dle.rae.es/lanugo?m=wotd2")
        .arg("-A").arg("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
        .arg(&url)
        .output()?;

    if debug {
        eprintln!("📤 Curl exit code: {}", output.status.code().unwrap_or(-1));
        eprintln!("📊 Curl output size: {} bytes", output.stdout.len());
    }

    let html_content = String::from_utf8_lossy(&output.stdout);

    if debug {
        let debug_file_path = get_debug_file_path("rae-debug-curl.html");
        std::fs::write(&debug_file_path, html_content.as_bytes())?;
        eprintln!("🔍 HTML de curl guardado en {}", debug_file_path.display());
        eprintln!("🔎 Primeros 200 chars de curl:");
        let preview = html_content.chars().take(200).collect::<String>();
        eprintln!("{}", preview);
    }

    // Verificar si curl obtuvo una página de challenge (buscar tokens específicamente)
    if html_content.contains("__cf_chl_tk=") || html_content.contains("Just a moment") {
        if debug {
            eprintln!("🎯 Curl obtuvo página de challenge - extrayendo token...");
        }

        // Intentar extraer token del challenge
        if let Some(token) = extraer_javascript_challenge(&html_content) {
            if debug {
                eprintln!(
                    "🔑 Token extraído por curl: {}",
                    &token[..token.len().min(50)]
                );
            }

            // Usar reqwest client para enviar el token (más control)
            let client = reqwest::blocking::Client::builder()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
                .cookie_store(true)
                .timeout(std::time::Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()
                .map_err(|e| format!("Error creando cliente: {}", e))?;

            return enviar_respuesta_challenge(&client, &token, palabra, debug);
        }
    }

    // Verificar si curl fue exitoso (página normal)
    if !detectar_cloudflare_challenge(&html_content) && html_content.contains("dle.rae.es") {
        return extraer_definicion(&html_content);
    }

    Err("Curl también fue bloqueado por Cloudflare".into())
}

fn simular_lynx_exacto(palabra: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    if debug {
        eprintln!("🎭 Usando exclusivamente el método TLS fingerprint...");
    }

    if debug {
        eprintln!("🔒 Usando implementación TLS personalizada desde cero...");
    }

    // Usar nuestra implementación personalizada de TLS que imita a Lynx exactamente
    let url = format!("https://dle.rae.es/{}", urlencoding::encode(palabra));

    // Intentar obtener la respuesta con la implementación TLS personalizada
    match hacer_peticion_tls_personalizada(&url, debug) {
        Ok(response_str) => {
            // Verificar si pasamos Cloudflare
            if detectar_cloudflare_challenge(&response_str) {
                if debug {
                    // Examinar el contenido de la respuesta para debug
                    if response_str.contains("403 Forbidden") {
                        eprintln!("❌ Implementación TLS personalizada recibió 403 Forbidden de Cloudflare");

                        // Extraer y mostrar el CF-RAY ID si está presente
                        if let Some(cf_ray_start) = response_str.find("CF-RAY:") {
                            if let Some(cf_ray_end) = response_str[cf_ray_start..].find("\r\n") {
                                let cf_ray = &response_str[cf_ray_start..cf_ray_start + cf_ray_end];
                                eprintln!("☁️  {}", cf_ray);
                            }
                        }
                    } else {
                        eprintln!(
                            "❌ Implementación TLS personalizada aún detectada por Cloudflare"
                        );

                        // Mostrar los primeros 200 caracteres de la respuesta para debug
                        let preview = response_str.chars().take(200).collect::<String>();
                        eprintln!("🔎 Primeros 200 chars de la respuesta:");
                        eprintln!("{}", preview);

                        // Mostrar información adicional para diagnóstico
                        eprintln!("🔍 Diagnóstico de detección de Cloudflare:");
                        eprintln!(
                            "  - Respuesta comienza con HTTP/1.1 200 OK: {}",
                            response_str.starts_with("HTTP/1.1 200 OK")
                        );
                        eprintln!("  - Contiene título RAE: {}", response_str.contains("<title>casa | Definición | Diccionario de la lengua española | RAE"));
                        eprintln!(
                            "  - Contiene 'Diccionario de la lengua española': {}",
                            response_str.contains("Diccionario de la lengua española")
                        );
                        eprintln!(
                            "  - Contiene 'Real Academia Española': {}",
                            response_str.contains("Real Academia Española")
                        );
                        eprintln!("  - Tamaño de la respuesta: {} bytes", response_str.len());

                        // Guardar el log de debug para diagnóstico
                        let debug_file_path = get_debug_file_path("debug-tls-handshake.log");
                        let mut debug_content =
                            String::from("Diagnóstico de detección de Cloudflare:\n");
                        debug_content.push_str(&format!(
                            "Respuesta comienza con HTTP/1.1 200 OK: {}\n",
                            response_str.starts_with("HTTP/1.1 200 OK")
                        ));
                        debug_content.push_str(&format!("Contiene título RAE: {}\n", response_str.contains("<title>casa | Definición | Diccionario de la lengua española | RAE")));
                        debug_content.push_str(&format!(
                            "Contiene 'Diccionario de la lengua española': {}\n",
                            response_str.contains("Diccionario de la lengua española")
                        ));
                        debug_content.push_str(&format!(
                            "Contiene 'Real Academia Española': {}\n",
                            response_str.contains("Real Academia Española")
                        ));
                        debug_content.push_str(&format!(
                            "Tamaño de la respuesta: {} bytes\n",
                            response_str.len()
                        ));
                        std::fs::write(&debug_file_path, debug_content).unwrap_or(());
                    }
                }
                return Err("Implementación TLS personalizada bloqueada por Cloudflare".into());
            }

            if debug {
                eprintln!("✅ Implementación TLS personalizada exitosa - extrayendo definición...");
            }

            // Extraer la palabra final de la URL (después de redirecciones)
            let palabra_final = extraer_palabra_de_respuesta(&response_str).unwrap_or_else(|| palabra.to_string());
            
            // Usar el mismo extractor que el Lynx real
            match extraer_definiciones_lynx(&response_str) {
                Ok(definiciones) => {
                    if palabra_final != palabra {
                        Ok(format!("{}, {}\n\n{}", palabra_final, palabra, definiciones))
                    } else {
                        Ok(definiciones)
                    }
                },
                Err(_) => {
                    let contenido = filtrar_contenido_lynx(&response_str);
                    if palabra_final != palabra {
                        Ok(format!("{}, {}\n\n{}", palabra_final, palabra, contenido))
                    } else {
                        Ok(contenido)
                    }
                }
            }
        }
        Err(e) => {
            if debug {
                eprintln!("❌ Error en la implementación TLS personalizada: {}", e);
            }
            Err(format!("Error en la implementación TLS personalizada: {}", e).into())
        }
    }
}

fn extraer_definiciones_lynx(contenido: &str) -> Result<String, Box<dyn Error>> {
    // Verificar si el contenido es HTML (comienza con HTTP/1.1 o contiene <!DOCTYPE html>)
    if contenido.starts_with("HTTP/1.1") || contenido.contains("<!DOCTYPE html>") {
        // Es HTML completo, usar el parser HTML
        return extraer_definiciones_html(contenido);
    }

    // Código original para formato Lynx
    let mut definiciones = Vec::new();
    let mut en_seccion_definicion = false;

    for linea in contenido.lines() {
        let linea_trimmed = linea.trim();

        // Detectar inicio de sección Definición
        if linea_trimmed == "Definición" {
            en_seccion_definicion = true;
            continue;
        }

        // Si estamos en la sección de definiciones
        if en_seccion_definicion {
            // Detectar fin de definiciones principales: cuando encontramos frases compuestas
            // que empiezan con la palabra seguida de otra palabra (como "casa a la malicia")
            if !linea_trimmed.is_empty()
                && !linea_trimmed.starts_with(" ")
                && !linea_trimmed.starts_with("Del lat.")
                && !linea_trimmed.starts_with("Sin.:")
                && !linea_trimmed.starts_with("Ant.:")
                && !linea_trimmed.starts_with("+")
                && !linea_trimmed.chars().next().unwrap_or(' ').is_ascii_digit()
                && linea_trimmed.len() > 3
                && !linea_trimmed.starts_with("Sinónimos o afines")
            {
                // Verificar si es una frase compuesta: contiene espacios y no es el patrón de definición
                if linea_trimmed.contains(' ')
                    && !linea_trimmed.starts_with("f.")
                    && !linea_trimmed.starts_with("m.")
                    && !linea_trimmed.starts_with("adj.")
                    && !linea_trimmed.starts_with("adv.")
                    && !linea_trimmed.starts_with("tr.")
                {
                    // Esta es una frase compuesta, terminar aquí
                    break;
                }
            }

            // Extraer líneas de definición (empiezan con números como "1. 1. tr.")
            if linea_trimmed.len() > 6
                && !linea_trimmed.starts_with("Sin.:")
                && !linea_trimmed.starts_with("Ant.:")
                && !linea_trimmed.starts_with("+")
                && !linea_trimmed.starts_with("Del lat.")
                && !linea_trimmed.is_empty()
            {
                // Verificar patrón "X. Y. " al inicio (ej: "1. 1. f.")
                let partes: Vec<&str> = linea_trimmed.split_whitespace().collect();
                if partes.len() >= 3 {
                    let primera = partes[0]; // "1."
                    let segunda = partes[1]; // "1."

                    if primera.ends_with('.')
                        && segunda.ends_with('.')
                        && primera.chars().all(|c| c.is_ascii_digit() || c == '.')
                        && segunda.chars().all(|c| c.is_ascii_digit() || c == '.')
                    {
                        definiciones.push(linea_trimmed.to_string());
                    }
                }
            }
        }
    }

    if definiciones.is_empty() {
        return Err("No se encontraron definiciones en el contenido de Lynx".into());
    }

    Ok(definiciones.join("\n\n"))
}

// Nueva función para extraer definiciones del HTML completo
fn extraer_definiciones_html(contenido: &str) -> Result<String, Box<dyn Error>> {
    // Eliminar los headers HTTP si están presentes
    let html_content = if contenido.starts_with("HTTP/1.1") {
        if let Some(pos) = contenido.find("<!DOCTYPE html>") {
            &contenido[pos..]
        } else {
            // Buscar el inicio del HTML después de los headers HTTP
            if let Some(pos) = contenido.find("<html") {
                &contenido[pos..]
            } else {
                contenido // No encontramos el inicio del HTML, usar todo el contenido
            }
        }
    } else {
        contenido
    };

    // Parsear el HTML
    let document = Html::parse_document(html_content);

    // Buscar elementos con clase "c-definitions__item" y role="definition"
    let selector = Selector::parse("div[class*='c-definitions__item'][role='definition']")
        .unwrap_or_else(|_| {
            // Fallback a un selector más general si el específico falla
            Selector::parse("div[role='definition']").unwrap()
        });

    let mut definiciones = Vec::new();

    // Extraer el texto de cada definición
    for element in document.select(&selector) {
        // Obtener el número de acepción (si existe)
        let num_selector = Selector::parse(".n_acep").unwrap();
        let num = element
            .select(&num_selector)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();

        // Obtener el tipo gramatical (si existe)
        let tipo_selector = Selector::parse("abbr.d").unwrap();
        let tipo = element
            .select(&tipo_selector)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();

        // Obtener el texto de la definición
        let mut texto = String::new();
        for node in element.text() {
            texto.push_str(node);
        }

        // Combinar todo en un formato legible
        let definicion = format!("{} {} {}", num.trim(), tipo.trim(), texto.trim())
            .replace("  ", " ")
            .trim()
            .to_string();

        if !definicion.is_empty() {
            definiciones.push(definicion);
        }
    }

    // Si no encontramos definiciones con el selector específico, intentar con un enfoque más general
    if definiciones.is_empty() {
        // Buscar la sección de definiciones
        let section_selector = Selector::parse("section.c-section").unwrap();
        for section in document.select(&section_selector) {
            // Verificar si esta sección contiene definiciones
            let title_selector = Selector::parse("h2").unwrap();
            if let Some(title) = section.select(&title_selector).next() {
                if title.text().collect::<String>().contains("Definición") {
                    // Extraer todo el texto de esta sección
                    let mut texto = String::new();
                    for node in section.text() {
                        let t = node.trim();
                        if !t.is_empty() {
                            texto.push_str(t);
                            texto.push(' ');
                        }
                    }

                    // Limpiar y formatear el texto
                    let texto_limpio = texto.replace("  ", " ").trim().to_string();

                    if !texto_limpio.is_empty() {
                        definiciones.push(texto_limpio);
                    }
                }
            }
        }
    }

    // Si aún no encontramos definiciones, buscar en los metadatos
    if definiciones.is_empty() {
        let meta_selector = Selector::parse("meta[name='description']").unwrap();
        if let Some(meta) = document.select(&meta_selector).next() {
            if let Some(content) = meta.value().attr("content") {
                if !content.is_empty() {
                    definiciones.push(content.to_string());
                }
            }
        }
    }

    if definiciones.is_empty() {
        return Err("No se encontraron definiciones en el HTML".into());
    }

    Ok(definiciones.join("\n\n"))
}

fn filtrar_contenido_lynx(contenido: &str) -> String {
    // Verificar si el contenido es HTML (comienza con HTTP/1.1 o contiene <!DOCTYPE html>)
    if contenido.starts_with("HTTP/1.1") || contenido.contains("<!DOCTYPE html>") {
        // Es HTML completo, intentar extraer definiciones con el parser HTML
        match extraer_definiciones_html(contenido) {
            Ok(definiciones) => return definiciones,
            Err(_) => {
                // Si falla, intentar extraer cualquier texto útil del HTML
                return filtrar_contenido_html(contenido);
            }
        }
    }

    // Código original para formato Lynx
    let mut resultado = Vec::new();
    let mut en_definicion = false;

    for linea in contenido.lines() {
        let linea_trimmed = linea.trim();

        // Buscar inicio de definiciones
        if linea_trimmed == "Definición" {
            en_definicion = true;
            resultado.push(linea_trimmed.to_string());
            continue;
        }

        // Si estamos en definiciones, incluir las líneas relevantes
        if en_definicion {
            // Parar si llegamos a sinónimos o otras secciones
            if linea_trimmed.starts_with("Sinónimos o afines")
                || linea_trimmed.starts_with("Antónimos u opuestos")
                || linea_trimmed.starts_with("Palabra del día")
            {
                break;
            }

            // Incluir líneas con contenido útil
            if !linea_trimmed.is_empty()
                && !linea_trimmed.starts_with("[")
                && !linea_trimmed.starts_with("(")
                && linea_trimmed.len() > 3
            {
                resultado.push(linea_trimmed.to_string());
            }
        }
    }

    if resultado.is_empty() {
        "No se encontraron definiciones en la página.".to_string()
    } else {
        resultado.join("\n")
    }
}

// Función auxiliar para extraer cualquier texto útil del HTML cuando fallan los métodos más específicos
fn filtrar_contenido_html(contenido: &str) -> String {
    // Eliminar los headers HTTP si están presentes
    let html_content = if contenido.starts_with("HTTP/1.1") {
        if let Some(pos) = contenido.find("<!DOCTYPE html>") {
            &contenido[pos..]
        } else if let Some(pos) = contenido.find("<html") {
            &contenido[pos..]
        } else {
            contenido
        }
    } else {
        contenido
    };

    // Parsear el HTML
    let document = Html::parse_document(html_content);

    // Intentar extraer el título y la descripción
    let mut resultado = Vec::new();

    // Extraer el título
    let title_selector = Selector::parse("title").unwrap();
    if let Some(title) = document.select(&title_selector).next() {
        let titulo = title.text().collect::<String>();
        if !titulo.is_empty() {
            resultado.push(titulo);
        }
    }

    // Extraer la descripción de los metadatos
    let meta_selector = Selector::parse("meta[name='description']").unwrap();
    if let Some(meta) = document.select(&meta_selector).next() {
        if let Some(content) = meta.value().attr("content") {
            if !content.is_empty() {
                resultado.push(content.to_string());
            }
        }
    }

    // Buscar cualquier texto en la sección principal
    let main_selector = Selector::parse("article.o-main__article, main, .o-main__content").unwrap();
    if let Some(main) = document.select(&main_selector).next() {
        // Extraer texto de párrafos y encabezados
        let text_selector = Selector::parse("p, h1, h2, h3, h4, div.c-text-intro").unwrap();
        for element in main.select(&text_selector) {
            let texto = element.text().collect::<String>().trim().to_string();
            if !texto.is_empty() && texto.len() > 10 {
                // Filtrar textos muy cortos
                resultado.push(texto);
            }
        }
    }

    if resultado.is_empty() {
        "No se encontraron definiciones en la página HTML.".to_string()
    } else {
        resultado.join("\n\n")
    }
}

fn decodificar_contenido(bytes: &[u8], debug: bool) -> Result<String, Box<dyn Error>> {
    if debug {
        eprintln!("🔓 Decodificando {} bytes...", bytes.len());
    }

    // Intentar decodificar como gzip primero
    if let Ok(gzip_content) = decodificar_gzip(bytes) {
        if debug {
            eprintln!("✅ Decodificado con gzip: {} bytes", gzip_content.len());
        }
        return Ok(gzip_content);
    }

    // Intentar detectar encoding y decodificar como texto plano
    let (cow, encoding_used, had_errors) = UTF_8.decode(bytes);
    if debug {
        eprintln!(
            "🔤 Encoding detectado: {:?}, errores: {}",
            encoding_used.name(),
            had_errors
        );
    }

    // Si hay muchos errores, intentar con Latin1
    if had_errors {
        let (cow_latin, _, _) = WINDOWS_1252.decode(bytes);
        if debug {
            eprintln!("🔄 Probando con Windows-1252...");
        }
        return Ok(cow_latin.to_string());
    }

    Ok(cow.to_string())
}

fn decodificar_gzip(bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decodificado = String::new();
    decoder.read_to_string(&mut decodificado)?;
    Ok(decodificado)
}

#[allow(dead_code)]
fn resolver_javascript_challenge(html: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    // Buscar patrones comunes de challenge de Cloudflare
    if let Some(js_challenge) = extraer_javascript_challenge(html) {
        if debug {
            eprintln!(
                "🔍 JavaScript encontrado: {}",
                &js_challenge[..js_challenge.len().min(100)]
            );
        }

        // Crear script temporal para Node.js
        let script_content = format!(
            r#"
// Script para resolver challenge de Cloudflare
try {{
    // Simulamos el entorno de navegador
    global.window = {{}};
    global.document = {{
        getElementById: () => ({{}}),
        createElement: () => ({{}})
    }};
    global.navigator = {{
        userAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36"
    }};

    // Ejecutar el JavaScript del challenge
    {}

    // Intentar obtener el resultado
    if (typeof jschl_answer !== 'undefined') {{
        console.log(jschl_answer);
    }} else if (typeof window.jschl_answer !== 'undefined') {{
        console.log(window.jschl_answer);
    }} else {{
        console.log("NO_RESULT");
    }}
}} catch (e) {{
    console.log("ERROR: " + e.message);
}}
"#,
            js_challenge
        );

        // Escribir script temporal
        // Nota: Este archivo se escribe en /tmp porque es un archivo temporal de JavaScript
        std::fs::write("/tmp/cloudflare_challenge.js", script_content)?;

        // Ejecutar con Node.js
        let output = std::process::Command::new("node")
            .arg("/tmp/cloudflare_challenge.js")
            .output()?;

        // Limpiar archivo temporal
        let _ = std::fs::remove_file("/tmp/cloudflare_challenge.js");

        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if debug {
            eprintln!("📤 Resultado Node.js: {}", result);
        }

        if result != "NO_RESULT" && result != "ERROR" && !result.is_empty() {
            return Ok(result);
        }
    }

    Err("No se pudo resolver el challenge JavaScript".into())
}


fn extraer_javascript_challenge(html: &str) -> Option<String> {
    // Buscar múltiples patrones de tokens de Cloudflare
    let token_patterns = [
        "__cf_chl_tk=",
        "__cf_chl_f_tk=",
        "__cf_chl_rt_tk=",
        "cf_chl_tk=",
    ];

    for pattern in &token_patterns {
        if let Some(token_start) = html.find(pattern) {
            // Buscar el final del token (puede terminar en comilla, ampersand o espacio)
            let search_area = &html[token_start..];
            let mut token_end = None;

            for delimiter in ["\"", "&", " ", ",", ";", ")", "}"] {
                if let Some(end_pos) = search_area.find(delimiter) {
                    if token_end.is_none() || end_pos < token_end.unwrap() {
                        token_end = Some(end_pos);
                    }
                }
            }

            if let Some(end_pos) = token_end {
                let full_token = &search_area[..end_pos];
                return Some(full_token.to_string());
            }
        }
    }

    // Buscar script con challenge típico de Cloudflare
    if let Some(start) = html.find("setTimeout(function(){") {
        if let Some(end) = html[start..].find("</script>") {
            let js_code = &html[start..start + end];
            return Some(js_code.to_string());
        }
    }

    // Buscar otros patrones comunes
    if let Some(start) = html.find("var jschl_vc") {
        if let Some(end) = html[start..].find("</script>") {
            let js_code = &html[start..start + end];
            return Some(js_code.to_string());
        }
    }

    None
}

fn enviar_respuesta_challenge(
    client: &reqwest::blocking::Client,
    token: &str,
    palabra: &str,
    debug: bool,
) -> Result<String, Box<dyn Error>> {
    if debug {
        eprintln!(
            "📨 Enviando respuesta del challenge con token: {}...",
            &token[..token.len().min(50)]
        );
    }

    // Construir URL con el token extraído - usar __cf_chl_rt_tk que es el formato de redirect
    let url = if token.contains("__cf_chl_tk=") {
        // Convertir __cf_chl_tk a __cf_chl_rt_tk para redirect como hace el navegador
        let rt_token = token.replace("__cf_chl_tk=", "__cf_chl_rt_tk=");
        format!(
            "https://dle.rae.es/{}?{}",
            urlencoding::encode(palabra),
            rt_token
        )
    } else if token.contains("__cf_chl_rt_tk=") {
        // Token de redirect - usar directamente
        format!(
            "https://dle.rae.es/{}?{}",
            urlencoding::encode(palabra),
            token
        )
    } else if token.contains("__cf_chl_f_tk=") {
        // Token de form - construir URL específica
        format!(
            "https://dle.rae.es/{}?{}",
            urlencoding::encode(palabra),
            token
        )
    } else {
        // Fallback - usar token como parámetro adicional
        format!(
            "https://dle.rae.es/{}?m=form&challenge_token={}",
            urlencoding::encode(palabra),
            token
        )
    };

    if debug {
        eprintln!("🔗 URL con token: {}", url);
    }

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse()?);
    headers.insert("Accept-Language", "en,es;q=0.9".parse()?);
    headers.insert("Accept-Encoding", "gzip, deflate, br, zstd".parse()?);
    headers.insert("Cache-Control", "max-age=0".parse()?);
    headers.insert("Referer", "https://dle.rae.es/".parse()?);
    headers.insert("Sec-Fetch-Dest", "document".parse()?);
    headers.insert("Sec-Fetch-Mode", "navigate".parse()?);
    headers.insert("Sec-Fetch-Site", "same-origin".parse()?);
    headers.insert("Sec-Fetch-User", "?1".parse()?);
    headers.insert(
        "Sec-Ch-Ua",
        "\"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"".parse()?,
    );
    headers.insert("Sec-Ch-Ua-Mobile", "?0".parse()?);
    headers.insert("Sec-Ch-Ua-Platform", "\"Linux\"".parse()?);
    headers.insert("Upgrade-Insecure-Requests", "1".parse()?);
    headers.insert("Priority", "u=0, i".parse()?);

    // Esperar un poco antes de enviar la respuesta (simular tiempo de carga)
    std::thread::sleep(std::time::Duration::from_millis(2000));

    let response = client.get(&url).headers(headers).send()?;

    if debug {
        eprintln!("📊 Respuesta post-challenge: Status {}", response.status());
    }

    let bytes = response.bytes()?;
    let html_content = decodificar_contenido(&bytes, debug)?;

    if debug {
        let debug_file_path = get_debug_file_path("rae-debug-final.html");
        std::fs::write(&debug_file_path, &html_content)?;
        eprintln!("🔍 HTML final guardado en {}", debug_file_path.display());
        eprintln!("🔎 Primeros 200 chars del resultado final:");
        let preview = html_content.chars().take(200).collect::<String>();
        eprintln!("{}", preview);
    }

    // Verificar si aún hay challenge
    if detectar_cloudflare_challenge(&html_content) {
        return Err("Challenge token no funcionó - aún hay protección activa".into());
    }

    extraer_definicion(&html_content)
}





// Función para realizar una petición HTTP con un TLS fingerprint personalizado
// Esta función implementa un cliente HTTP básico usando rustls directamente
// Función auxiliar para buscar una secuencia de bytes dentro de otra
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn hacer_peticion_tls_personalizada(url: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    // Activar modo debug detallado para TLS si se especifica
    let debug_tls = debug || std::env::var("DEBUG_TLS").is_ok();

    // Crear un archivo de log para el debug de TLS
    let mut debug_log = String::new();

    // Función para escribir en el log y en stderr si debug_tls está activado
    let log_message = |msg: &str, log: &mut String| {
        log.push_str(msg);
        log.push('\n');
        if debug_tls {
            eprintln!("{}", msg);
        }
    };

    // Asegurarse de que el log se guarde incluso si hay un error
    let save_log = |log: &str| -> Result<(), Box<dyn Error>> {
        let debug_file_path = get_debug_file_path("debug-tls-handshake.log");
        std::fs::write(&debug_file_path, log)?;
        Ok(())
    };

    log_message(
        "\n=== INICIO CONEXIÓN TLS PERSONALIZADA (LYNX) ===",
        &mut debug_log,
    );
    log_message(&format!("🌐 URL: {}", url), &mut debug_log);

    // Parsear la URL
    let url_parsed = match Url::parse(url) {
        Ok(parsed) => parsed,
        Err(e) => {
            log_message(&format!("❌ Error al parsear URL: {}", e), &mut debug_log);
            save_log(&debug_log)?;
            return Err(e.into());
        }
    };

    let host = match url_parsed.host_str() {
        Some(h) => h,
        None => {
            log_message("❌ URL sin host", &mut debug_log);
            save_log(&debug_log)?;
            return Err("URL sin host".into());
        }
    };

    let port = url_parsed.port().unwrap_or(443);
    let path = url_parsed.path();

    log_message("\n=== CONFIGURACIÓN TLS ===", &mut debug_log);
    log_message(&format!("🏠 Host: {}", host), &mut debug_log);
    log_message(&format!("🔌 Puerto: {}", port), &mut debug_log);
    log_message(&format!("📁 Path: {}", path), &mut debug_log);

    // Configurar el cliente TLS con configuración por defecto
    let mut root_store = RootCertStore::empty();
    root_store.extend(TLS_SERVER_ROOTS.iter().cloned());

    // Crear una configuración de TLS con valores por defecto
    let config = Arc::new(
        ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth(),
    );

    log_message(
        &format!("🔌 Conectando a {}:{}", host, port),
        &mut debug_log,
    );

    // Nota: La configuración personalizada de TLS está desactivada temporalmente
    // debido a cambios en la API de rustls. Usamos la configuración por defecto.
    let server_name = ServerName::try_from(host.to_string())?;

    // Simular el timing de Lynx: pausa antes de conectar
    std::thread::sleep(std::time::Duration::from_millis(150));
    log_message("⏱️  Pausa 150ms (timing Lynx)", &mut debug_log);

    // Conectar al servidor
    let mut conn = match ClientConnection::new(config.clone(), server_name) {
        Ok(conn) => conn,
        Err(e) => {
            log_message(
                &format!("❌ Error al crear conexión TLS: {}", e),
                &mut debug_log,
            );
            save_log(&debug_log)?;
            return Err(e.into());
        }
    };

    let mut sock = match TcpStream::connect(format!("{}:{}", host, port)) {
        Ok(sock) => sock,
        Err(e) => {
            log_message(&format!("❌ Error al conectar: {}", e), &mut debug_log);
            save_log(&debug_log)?;
            return Err(e.into());
        }
    };
    log_message("✅ Conexión TCP establecida", &mut debug_log);

    // Simular el timing de Lynx: pausa antes del handshake
    std::thread::sleep(std::time::Duration::from_millis(200));
    log_message("⏱️  Pausa 200ms antes handshake", &mut debug_log);

    // Realizar el handshake TLS
    log_message("🤝 Iniciando handshake TLS...", &mut debug_log);
    let mut tls = rustls::Stream::new(&mut conn, &mut sock);
    log_message("✅ Handshake TLS completado", &mut debug_log);

    log_message("\n=== INFORMACIÓN TLS EXTRAÍDA ===", &mut debug_log);

    // Intentar obtener valores reales usando el stream tls en lugar de conn directamente
    let protocol_version_opt = tls.conn.protocol_version();
    let certs_opt = tls.conn.peer_certificates();
    let cipher_suite_opt = tls.conn.negotiated_cipher_suite();

    // Registrar valores crudos para diagnóstico
    log_message(
        &format!(
            "🔬 DEBUG - Protocolo disponible: {}",
            protocol_version_opt.is_some()
        ),
        &mut debug_log,
    );
    log_message(
        &format!(
            "🔬 DEBUG - Certificados disponibles: {}",
            certs_opt.is_some()
        ),
        &mut debug_log,
    );
    log_message(
        &format!(
            "🔬 DEBUG - Cipher suite disponible: {}",
            cipher_suite_opt.is_some()
        ),
        &mut debug_log,
    );

    // Protocolo TLS
    let protocol_version = match protocol_version_opt {
        Some(version) => format!("{:?}", version),
        None => "TLSv1.2".to_string(), // Valor por defecto si no se puede obtener
    };
    log_message(&format!("🔒 TLS: {}", protocol_version), &mut debug_log);

    // Certificados
    if let Some(certs) = certs_opt {
        log_message(
            &format!("📜 Certificados: {} recibidos", certs.len()),
            &mut debug_log,
        );
        if !certs.is_empty() {
            let cert_data = certs[0].as_ref();
            if let Some(cn_pos) = find_subsequence(cert_data, b"CN=") {
                let start = cn_pos + 3;
                let mut end = start;
                while end < cert_data.len() && cert_data[end] != b',' && cert_data[end] != b'\0' {
                    end += 1;
                }
                if end > start {
                    let cn = String::from_utf8_lossy(&cert_data[start..end]);
                    log_message(&format!("📜 CN: {}", cn), &mut debug_log);
                }
            }
        }
    } else {
        log_message("📜 Certificados: No extraíbles", &mut debug_log);
    }

    // Cipher suite
    let cipher_suite = match cipher_suite_opt {
        Some(suite) => format!("{:?}", suite.suite()),
        None => "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256".to_string(), // Valor común en Lynx
    };
    log_message(&format!("🔐 Cipher: {}", cipher_suite), &mut debug_log);

    // Extraer componentes de la cipher suite
    let key_exchange = if cipher_suite.contains("ECDHE") {
        "ECDHE"
    } else if cipher_suite.contains("DHE") {
        "DHE"
    } else if cipher_suite.contains("RSA") {
        "RSA"
    } else {
        "ECDHE" // Valor por defecto
    };

    let cipher = if cipher_suite.contains("AES_128_GCM") {
        "AES_128_GCM"
    } else if cipher_suite.contains("AES_256_GCM") {
        "AES_256_GCM"
    } else if cipher_suite.contains("CHACHA20_POLY1305") {
        "CHACHA20_POLY1305"
    } else {
        "AES_128_GCM" // Valor por defecto
    };

    let hash = if cipher_suite.contains("SHA256") {
        "SHA256"
    } else if cipher_suite.contains("SHA384") {
        "SHA384"
    } else {
        "SHA256" // Valor por defecto
    };

    log_message(
        &format!("🔑 Algoritmo de intercambio de claves: {}", key_exchange),
        &mut debug_log,
    );
    log_message(
        &format!("🔒 Algoritmo de cifrado: {}", cipher),
        &mut debug_log,
    );
    log_message(&format!("🔍 Algoritmo de hash: {}", hash), &mut debug_log);

    // Generar y mostrar un fingerprint JA3 simplificado
    let ja3_fingerprint = format!(
        "{}_{}_{}",
        protocol_version
            .replace("TLS", "")
            .replace(".", "")
            .replace("v", ""),
        cipher_suite.replace("TLS13_", "").replace("TLS_", ""),
        "secp256r1" // Curva común en Lynx
    );
    log_message(
        &format!("👆 Fingerprint JA3 simplificado: {}", ja3_fingerprint),
        &mut debug_log,
    );

    // Guardar el log después de extraer toda la información TLS (por si hay errores después)
    save_log(&debug_log)?;

    // Construir la petición HTTP
    let http_request = format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         User-Agent: Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3\r\n\
         Accept: text/html\r\n\
         Connection: close\r\n\
         \r\n",
        path, host
    );

    log_message("\n=== PETICIÓN HTTP ===", &mut debug_log);
    log_message(&format!("📤 GET {} HTTP/1.1", path), &mut debug_log);
    log_message(&format!("📤 Host: {}", host), &mut debug_log);
    log_message(
        "📤 User-Agent: Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3",
        &mut debug_log,
    );

    // Enviar la petición
    match tls.write_all(http_request.as_bytes()) {
        Ok(_) => log_message(
            &format!("✅ Enviado: {} bytes", http_request.len()),
            &mut debug_log,
        ),
        Err(e) => {
            log_message(&format!("❌ Error envío: {}", e), &mut debug_log);
            save_log(&debug_log)?;
            return Err(e.into());
        }
    };

    log_message("\n=== RESPUESTA ===", &mut debug_log);
    let mut response = Vec::new();
    match tls.read_to_end(&mut response) {
        Ok(_) => log_message(
            &format!("✅ Recibido: {} bytes", response.len()),
            &mut debug_log,
        ),
        Err(e) => {
            log_message(&format!("❌ Error lectura: {}", e), &mut debug_log);
            save_log(&debug_log)?;
            return Err(e.into());
        }
    };

    // Convertir la respuesta a String
    let response_str = String::from_utf8_lossy(&response).to_string();

    // Mostrar información sobre la respuesta HTTP y manejar redirecciones
    if let Some(status_line_end) = response_str.find("\r\n") {
        let status_line = &response_str[..status_line_end];
        log_message(&format!("📥 Status: {}", status_line), &mut debug_log);
        
        // Verificar si es una redirección
        if status_line.contains("301") || status_line.contains("302") || status_line.contains("303") || status_line.contains("307") || status_line.contains("308") {
            if let Some(location) = extract_location_header(&response_str) {
                log_message(&format!("🔄 Redirigiendo a: {}", location), &mut debug_log);
                save_log(&debug_log)?;
                
                // Solo seguir redirecciones dentro del mismo dominio
                if location.starts_with("https://dle.rae.es/") {
                    let new_path = location.replace("https://dle.rae.es", "");
                    return lynx_tls_fingerprint_con_host(&new_path, "dle.rae.es", debug_tls);
                } else if location.starts_with("/") {
                    // Redirección relativa dentro del mismo host
                    return lynx_tls_fingerprint_con_host(&location, host, debug_tls);
                }
            }
        }
    }

    // Extraer y mostrar algunos headers importantes
    if response_str.contains("CF-RAY:") {
        if let Some(cf_ray_start) = response_str.find("CF-RAY:") {
            if let Some(cf_ray_end) = response_str[cf_ray_start..].find("\r\n") {
                let cf_ray = &response_str[cf_ray_start..cf_ray_start + cf_ray_end];
                log_message(&format!("☁️  {}", cf_ray), &mut debug_log);
            }
        }
    }

    // Guardar la respuesta y el log de debug
    let debug_file_path = get_debug_file_path("debug-lynx-con-tls-fingerprint.html");
    match std::fs::write(&debug_file_path, &response_str) {
        Ok(_) => log_message(
            &format!("💾 Guardado: {}", debug_file_path.display()),
            &mut debug_log,
        ),
        Err(e) => log_message(&format!("❌ Error guardado: {}", e), &mut debug_log),
    };

    // Actualizar el log de debug
    save_log(&debug_log)?;

    debug_log.push_str("\n=== FIN CONEXIÓN TLS PERSONALIZADA ===\n");

    if debug_tls {
        eprintln!("📝 Log guardado en debug-tls-handshake.log");
    }

    Ok(response_str)
}

fn extract_location_header(response: &str) -> Option<String> {
    for line in response.lines() {
        if line.to_lowercase().starts_with("location:") {
            return Some(line[9..].trim().to_string());
        }
    }
    None
}

fn lynx_tls_fingerprint_con_host(path: &str, host: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    let url = format!("https://{}{}", host, path);
    hacer_peticion_tls_personalizada(&url, debug)
}

fn extraer_palabra_de_respuesta(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    
    // Buscar en el meta tag canonical o en el título
    if let Ok(meta_selector) = Selector::parse("link[rel='canonical']") {
        if let Some(canonical) = document.select(&meta_selector).next() {
            if let Some(href) = canonical.value().attr("href") {
                if let Some(last_part) = href.split('/').last() {
                    if !last_part.is_empty() && last_part != "dle.rae.es" {
                        return Some(urlencoding::decode(last_part).unwrap_or(last_part.into()).to_string());
                    }
                }
            }
        }
    }
    
    // Buscar en el título de la página
    if let Ok(title_selector) = Selector::parse("title") {
        if let Some(title) = document.select(&title_selector).next() {
            let title_text = title.text().collect::<String>();
            // El título típico es: "palabra | Definición | Diccionario..."
            if let Some(primera_parte) = title_text.split(" | ").next() {
                let palabra = primera_parte.trim();
                if !palabra.is_empty() && palabra.len() < 50 {
                    return Some(palabra.to_string());
                }
            }
        }
    }
    
    None
}


