use clap::Parser;
use headless_chrome::{protocol::cdp::Page, Browser, LaunchOptions};
use scraper::{Html, Selector};
use std::error::Error;
use std::ffi::OsStr;
use colored::Colorize;
use std::io::Read;
use flate2::read::GzDecoder;
use encoding_rs::*;

fn mostrar_con_colores(definicion: &str) {
    let lineas: Vec<&str> = definicion.split('\n')
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
    
    /// Usar método HTTP simple en lugar de Chrome headless
    #[arg(long, help = "Usar método HTTP simple (más rápido, menos confiable)")]
    http: bool,
    
    /// Modo debug: guardar HTML recibido
    #[arg(long, help = "Guardar HTML recibido para debugging")]
    debug: bool,
    
    /// Modo análisis: probar diferentes configuraciones HTTP
    #[arg(long, help = "Analizar por qué Cloudflare bloquea diferentes configuraciones")]
    analizar: bool,
}

fn main() {
    let args = Args::parse();
    
    if args.analizar {
        analizar_deteccion_cloudflare(&args.palabra, args.debug);
        return;
    }
    
    let resultado = if args.http {
        // Intentar primero con simulación de Lynx
        match simular_lynx_exacto(&args.palabra, args.debug) {
            Ok(resultado) => Ok(resultado),
            Err(_) => {
                if args.debug {
                    eprintln!("⚠️  Simulación de Lynx falló, probando método HTTP tradicional...");
                }
                buscar_en_rae_http(&args.palabra, args.debug)
            }
        }
    } else {
        buscar_en_rae(&args.palabra)
    };
    
    match resultado {
        Ok(resultado) => mostrar_con_colores(&resultado),
        Err(e) => {
            if args.http {
                eprintln!("❌ Error con método HTTP: {}", e);
                eprintln!("💡 Intenta sin --http para usar Chrome headless");
            } else {
                eprintln!("❌ Error: {}", e);
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
    
    let response = client
        .get(&url)
        .headers(headers)
        .send()?;
    
    if debug { 
        eprintln!("\n=== RESPUESTA INICIAL ===");
        eprintln!("📊 Status: {}", response.status());
        eprintln!("📊 Headers recibidos: {:#?}", response.headers());
        eprintln!("📊 Tamaño del contenido: {} bytes", response.content_length().unwrap_or(0));
        if let Some(cf_ray) = response.headers().get("cf-ray") {
            eprintln!("☁️  Cloudflare Ray ID: {:?}", cf_ray);
        }
    }
    
    // Obtener bytes en lugar de texto para decodificar manualmente
    let bytes = response.bytes()?;
    let html_content = decodificar_contenido(&bytes, debug)?;
    
    if debug {
        std::fs::write("rae-debug-lynx.html", &html_content)?;
        eprintln!("\n=== ANÁLISIS DEL CONTENIDO ===");
        eprintln!("🔍 HTML guardado en rae-debug-lynx.html");
        eprintln!("📊 Tamaño: {} bytes", html_content.len());
        eprintln!("🔎 Primeros 200 chars:");
        let preview = html_content.chars().take(200).collect::<String>();
        eprintln!("{}", preview);
    }
    
    // Detectar si es una página de desafío de Cloudflare
    if detectar_cloudflare_challenge(&html_content) {
        if debug { eprintln!("\n🛡️  DETECTADO DESAFÍO DE CLOUDFLARE - iniciando resolución..."); }
        return intentar_resolver_challenge(&client, &html_content, palabra, debug);
    }
    
    if debug { eprintln!("\n✅ CONTENIDO VÁLIDO - extrayendo definición..."); }
    extraer_definicion(&html_content)
}

fn buscar_en_rae(palabra: &str) -> Result<String, Box<dyn Error>> {

    let options = LaunchOptions {
        headless: true, // Modo headless para ejecución silenciosa
        args: vec![
            // Argumentos para parecer más un navegador real
            OsStr::new("--no-sandbox"),
            OsStr::new("--disable-blink-features=AutomationControlled"),
            OsStr::new("--disable-dev-shm-usage"),
            OsStr::new("--disable-gpu"),
            OsStr::new("--no-first-run"),
            OsStr::new("--no-default-browser-check"),
            OsStr::new("--disable-background-timer-throttling"),
            OsStr::new("--disable-backgrounding-occluded-windows"),
            OsStr::new("--disable-renderer-backgrounding"),
            OsStr::new("--disable-features=TranslateUI"),
            OsStr::new("--disable-ipc-flooding-protection"),
            OsStr::new("--enable-features=NetworkService,NetworkServiceLogging"),
            OsStr::new("--enforce-webrtc-ip-permission-check"),
            OsStr::new("--force-webrtc-ip-handling-policy=default_public_interface_only"),
            // User agent más convincente
            OsStr::new("--user-agent=Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"),
        ],
        ..Default::default()
    };

    let browser = Browser::new(options)?;
    let tab = browser.new_tab()?;

    // Inyectamos un script para que Cloudflare no detecte el modo 'webdriver'.
    // Esta es la técnica clave para eludir la protección.
    tab.call_method(Page::AddScriptToEvaluateOnNewDocument {
        source: "Object.defineProperty(navigator, 'webdriver', { get: () => false, });".to_string(),
        world_name: None,
        include_command_line_api: Some(false),
        run_immediately: Some(true),
    })?;

    // Enmascar la automatización ejecutando JavaScript
    tab.evaluate("
        // Eliminar propiedades que delatan automatización
        Object.defineProperty(navigator, 'webdriver', {
            get: () => undefined,
        });

        // Agregar propiedades típicas de navegadores reales
        Object.defineProperty(navigator, 'languages', {
            get: () => ['es-ES', 'es', 'en'],
        });

        // Simular interacciones de ratón
        window.chrome = {
            runtime: {}
        };

        // Eliminar indicadores de headless
        Object.defineProperty(navigator, 'plugins', {
            get: () => [1, 2, 3, 4, 5],
        });
    ", false)?;

    let url = format!("https://dle.rae.es/{}", urlencoding::encode(palabra));
    tab.navigate_to(&url)?;

    match tab.wait_for_element_with_custom_timeout("article", std::time::Duration::from_secs(15)) {
        Ok(_) => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let html_content = tab.get_content()?;
            tab.close(true)?;
            extraer_definicion(&html_content)
        }
        Err(_) => {
            Err("".into())
        }
    }
}

fn extraer_definicion(html_content: &str) -> Result<String, Box<dyn Error>> {
    let document = Html::parse_document(html_content);

    // Selectores específicos para definiciones de la RAE
    let selectores_definicion = [
        "article p",
        "article ol li",
        "article ul li",
        ".normal p",
        ".normal li",
        "#resultados p",
        "#resultados li"
    ];

    let mut definiciones_encontradas = Vec::new();

    for selector_str in &selectores_definicion {
        if let Ok(selector) = Selector::parse(selector_str) {
            for elemento in document.select(&selector) {
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

    // Si no encontramos nada con los selectores específicos, buscar en todo el artículo
    if definiciones_encontradas.is_empty() {
        if let Ok(article_selector) = Selector::parse("article") {
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
        "antónimos"
    ];

    let texto_lower = texto.to_lowercase();
    for exclusion in &exclusiones {
        if texto_lower.contains(exclusion) {
            return false;
        }
    }

    // Patrones típicos de definiciones de la RAE

    // 1. Empieza con número y categoría gramatical: "1. m.", "2. f.", etc.
    if texto.chars().next().map_or(false, |c| c.is_ascii_digit()) && texto.contains(". ") {
        // Verificar si después del número hay categoría gramatical
        let categorias = ["m.", "f.", "adj.", "tr.", "intr.", "prnl.", "loc.", "adv.", "interj.", "prep.", "conj."];
        for categoria in &categorias {
            if texto.contains(categoria) {
                return true;
            }
        }
    }

    // 2. Empieza directamente con categoría gramatical
    let categorias_inicio = [
        "m. ", "f. ", "adj. ", "tr. ", "intr. ", "prnl. ", 
        "loc. ", "adv. ", "interj. ", "prep. ", "conj. ", "m. y f. "
    ];

    for categoria in &categorias_inicio {
        if texto.starts_with(categoria) {
            return true;
        }
    }

    false
}

fn detectar_cloudflare_challenge(html: &str) -> bool {
    // Detectar indicadores comunes de desafío de Cloudflare
    html.contains("Checking your browser") ||
    html.contains("Please wait") ||
    html.contains("cloudflare") ||
    html.contains("cf-challenge") ||
    html.contains("__cf_chl_jschl_tk__") ||
    html.contains("DDoS protection") ||
    html.len() < 5000 || // Páginas de desafío suelen ser pequeñas
    // Detectar contenido binario/comprimido (probable respuesta de Cloudflare)
    (html.len() < 10000 && html.chars().any(|c| c as u32 > 127))
}

fn intentar_resolver_challenge(
    client: &reqwest::blocking::Client, 
    html: &str, 
    palabra: &str, 
    debug: bool
) -> Result<String, Box<dyn Error>> {
    if debug { eprintln!("🧩 Intentando resolver challenge JavaScript..."); }
    
    // Intentar extraer y resolver challenge JavaScript
    if let Ok(challenge_response) = resolver_javascript_challenge(html, debug) {
        if debug { eprintln!("✅ Challenge resuelto: {}", challenge_response); }
        
        // Enviar respuesta del challenge
        return enviar_respuesta_challenge(client, &challenge_response, palabra, debug);
    }
    
    // NUEVO: Intentar simular Lynx exactamente (basado en análisis tcpdump)
    if debug { eprintln!("🎭 Intentando simular Lynx exactamente..."); }
    if let Ok(lynx_sim_result) = simular_lynx_exacto(palabra, debug) {
        return Ok(lynx_sim_result);
    }
    
    // NUEVO: Intentar con Lynx directamente (método que sabemos que funciona)
    if debug { eprintln!("🐱 Intentando con Lynx real como fallback..."); }
    if let Ok(lynx_result) = intentar_con_lynx(palabra, debug) {
        return Ok(lynx_result);
    }
    
    // Fallback: Intentar con curl
    if debug { eprintln!("🔧 Intentando con curl como última alternativa..."); }
    if let Ok(curl_result) = intentar_con_curl(palabra, debug) {
        return Ok(curl_result);
    }
    
    if debug { eprintln!("⏳ Fallback: Esperando 5 segundos (como haría un navegador)..."); }
    
    // Fallback: Esperar 5 segundos como requiere Cloudflare
    std::thread::sleep(std::time::Duration::from_secs(5));
    
    // Intentar segunda petición después de la espera
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8".parse()?);
    headers.insert("Accept-Language", "es-ES,es;q=0.9,en;q=0.8".parse()?);
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse()?);
    headers.insert("Referer", "https://dle.rae.es/".parse()?);
    headers.insert("Sec-Fetch-Dest", "document".parse()?);
    headers.insert("Sec-Fetch-Mode", "navigate".parse()?);
    headers.insert("Sec-Fetch-Site", "same-origin".parse()?);
    headers.insert("Sec-Ch-Ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"".parse()?);
    headers.insert("Sec-Ch-Ua-Mobile", "?0".parse()?);
    headers.insert("Sec-Ch-Ua-Platform", "\"Linux\"".parse()?);
    headers.insert("Upgrade-Insecure-Requests", "1".parse()?);
    
    if debug { eprintln!("🔄 Reintentando petición después de espera..."); }
    
    let url = format!("https://dle.rae.es/{}", urlencoding::encode(palabra));
    let response = client
        .get(&url)
        .headers(headers)
        .send()?;
    
    if debug { eprintln!("📊 Respuesta después de challenge: Status {}", response.status()); }
    
    let html_content = response.text()?;
    
    if debug {
        std::fs::write("rae-debug-after-challenge.html", &html_content)?;
        eprintln!("🔍 HTML post-challenge guardado en rae-debug-after-challenge.html");
    }
    
    // Verificar si aún hay desafío
    if detectar_cloudflare_challenge(&html_content) {
        return Err("Cloudflare challenge no resuelto automáticamente".into());
    }
    
    extraer_definicion(&html_content)
}

fn intentar_con_curl(palabra: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    if debug { eprintln!("🌐 Ejecutando curl con headers exactos del navegador..."); }
    
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
        std::fs::write("rae-debug-curl.html", html_content.as_bytes())?;
        eprintln!("🔍 HTML de curl guardado en rae-debug-curl.html");
        eprintln!("🔎 Primeros 200 chars de curl:");
        let preview = html_content.chars().take(200).collect::<String>();
        eprintln!("{}", preview);
    }
    
    // Verificar si curl obtuvo una página de challenge (buscar tokens específicamente)
    if html_content.contains("__cf_chl_tk=") || html_content.contains("Just a moment") {
        if debug { eprintln!("🎯 Curl obtuvo página de challenge - extrayendo token..."); }
        
        // Intentar extraer token del challenge
        if let Some(token) = extraer_javascript_challenge(&html_content) {
            if debug { eprintln!("🔑 Token extraído por curl: {}", &token[..token.len().min(50)]); }
            
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
    if debug { eprintln!("🎭 Simulando comportamiento exacto de Lynx basado en análisis tcpdump..."); }
    
    // Configurar cliente HTTP para simular exactamente a Lynx
    let client = reqwest::blocking::Client::builder()
        .user_agent("Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3")
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5)) // Lynx es más conservador
        // Simular el comportamiento de conexión TCP de Lynx
        .tcp_nodelay(false) // Lynx no usa TCP_NODELAY agresivamente
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(1) // Lynx mantiene pocas conexiones concurrentes
        .build()?;

    // Headers minimalistas EXACTOS como Lynx (basado en tcpdump)
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Host", "dle.rae.es".parse()?);
    headers.insert("Accept", "text/html".parse()?); // Solo text/html, nada más
    // NO incluir Accept-Language, Accept-Encoding ni otros headers modernos
    // Lynx real es extremadamente minimalista
    
    let url = format!("https://dle.rae.es/{}", urlencoding::encode(palabra));
    if debug { 
        eprintln!("🌐 URL simulación Lynx: {}", url);
        eprintln!("🎭 User-Agent: Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3");
        eprintln!("📋 Headers minimalistas: {:#?}", headers);
    }
    
    // Simular el timing de Lynx: pequeña pausa antes de conectar
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let response = client
        .get(&url)
        .headers(headers)
        .send()?;
    
    if debug { 
        eprintln!("📊 Status simulación Lynx: {}", response.status());
        eprintln!("📊 Headers recibidos: {:#?}", response.headers());
        if let Some(cf_ray) = response.headers().get("cf-ray") {
            eprintln!("☁️  Cloudflare Ray ID: {:?}", cf_ray);
        }
    }
    
    let html_content = response.text()?;
    
    if debug {
        std::fs::write("rae-debug-lynx-simulation.html", &html_content)?;
        eprintln!("🔍 HTML simulación guardado en rae-debug-lynx-simulation.html");
        eprintln!("📊 Tamaño: {} bytes", html_content.len());
    }
    
    // Verificar si pasamos Cloudflare
    if detectar_cloudflare_challenge(&html_content) {
        if debug { eprintln!("❌ Simulación de Lynx aún detectada por Cloudflare"); }
        return Err("Simulación de Lynx bloqueada por Cloudflare".into());
    }
    
    if debug { eprintln!("✅ Simulación de Lynx exitosa - extrayendo definición..."); }
    
    // Usar el mismo extractor que el Lynx real
    match extraer_definiciones_lynx(&html_content) {
        Ok(definiciones) => Ok(definiciones),
        Err(_) => Ok(filtrar_contenido_lynx(&html_content))
    }
}

fn intentar_con_lynx(palabra: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    if debug { eprintln!("🐱 Ejecutando Lynx directamente (método que sabemos que funciona)..."); }
    
    let url = format!("https://dle.rae.es/{}", urlencoding::encode(palabra));
    
    // Usar Lynx con opciones correctas que pasan Cloudflare
    let output = std::process::Command::new("lynx")
        .arg("-trace")             // Traza de conexión
        .arg("-accept_all_cookies") // Aceptar todas las cookies
        .arg("-dump")              // Solo texto plano
        .arg(&url)
        .output()?;
    
    if debug { 
        eprintln!("📤 Lynx exit code: {}", output.status.code().unwrap_or(-1));
        eprintln!("📊 Lynx output size: {} bytes", output.stdout.len());
        if !output.stderr.is_empty() {
            eprintln!("⚠️  Lynx stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
    }
    
    if !output.status.success() {
        return Err(format!("Lynx falló con código: {}", output.status.code().unwrap_or(-1)).into());
    }
    
    let html_content = String::from_utf8_lossy(&output.stdout).to_string();
    
    if debug {
        std::fs::write("rae-debug-lynx-direct.txt", &html_content)?;
        eprintln!("🔍 Salida de Lynx guardada en rae-debug-lynx-direct.txt");
        eprintln!("🔎 Primeros 200 chars de Lynx:");
        let preview = html_content.chars().take(200).collect::<String>();
        eprintln!("{}", preview);
    }
    
    // Verificar si contiene contenido válido de la RAE
    if html_content.contains("Real Academia Española") || 
       html_content.contains("diccionario") ||
       html_content.len() > 500 {  // Contenido sustancial
        
        // Extraer solo las definiciones del texto plano de Lynx
        match extraer_definiciones_lynx(&html_content) {
            Ok(definiciones) => return Ok(definiciones),
            Err(e) => {
                if debug { eprintln!("⚠️  Error extrayendo definiciones de Lynx: {}", e); }
                // Si no podemos extraer definiciones específicas, devolver todo el contenido
                // filtrado para mostrar al menos algo útil
                return Ok(filtrar_contenido_lynx(&html_content));
            }
        }
    }
    
    Err("Lynx no obtuvo contenido válido de la RAE".into())
}

fn extraer_definiciones_lynx(contenido: &str) -> Result<String, Box<dyn Error>> {
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
            if !linea_trimmed.is_empty() && 
               !linea_trimmed.starts_with(" ") &&
               !linea_trimmed.starts_with("Del lat.") &&
               !linea_trimmed.starts_with("Sin.:") &&
               !linea_trimmed.starts_with("Ant.:") &&
               !linea_trimmed.starts_with("+") &&
               !linea_trimmed.chars().next().unwrap_or(' ').is_ascii_digit() &&
               linea_trimmed.len() > 3 &&
               !linea_trimmed.starts_with("Sinónimos o afines") {
                
                // Verificar si es una frase compuesta: contiene espacios y no es el patrón de definición
                if linea_trimmed.contains(' ') && 
                   !linea_trimmed.starts_with("f.") &&
                   !linea_trimmed.starts_with("m.") &&
                   !linea_trimmed.starts_with("adj.") &&
                   !linea_trimmed.starts_with("adv.") &&
                   !linea_trimmed.starts_with("tr.") {
                    // Esta es una frase compuesta, terminar aquí
                    break;
                }
            }
            
            // Extraer líneas de definición (empiezan con números como "1. 1. tr.")
            if linea_trimmed.len() > 6 && 
               !linea_trimmed.starts_with("Sin.:") &&
               !linea_trimmed.starts_with("Ant.:") &&
               !linea_trimmed.starts_with("+") &&
               !linea_trimmed.starts_with("Del lat.") &&
               !linea_trimmed.is_empty() {
                
                // Verificar patrón "X. Y. " al inicio (ej: "1. 1. f.")
                let partes: Vec<&str> = linea_trimmed.split_whitespace().collect();
                if partes.len() >= 3 {
                    let primera = partes[0];  // "1."
                    let segunda = partes[1];  // "1."
                    
                    if primera.ends_with('.') && segunda.ends_with('.') &&
                       primera.chars().all(|c| c.is_ascii_digit() || c == '.') &&
                       segunda.chars().all(|c| c.is_ascii_digit() || c == '.') {
                        
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

fn filtrar_contenido_lynx(contenido: &str) -> String {
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
            if linea_trimmed.starts_with("Sinónimos o afines") ||
               linea_trimmed.starts_with("Antónimos u opuestos") ||
               linea_trimmed.starts_with("Palabra del día") {
                break;
            }
            
            // Incluir líneas con contenido útil
            if !linea_trimmed.is_empty() && 
               !linea_trimmed.starts_with("[") &&
               !linea_trimmed.starts_with("(") &&
               linea_trimmed.len() > 3 {
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

fn decodificar_contenido(bytes: &[u8], debug: bool) -> Result<String, Box<dyn Error>> {
    if debug { eprintln!("🔓 Decodificando {} bytes...", bytes.len()); }
    
    // Intentar decodificar como gzip primero
    if let Ok(gzip_content) = decodificar_gzip(bytes) {
        if debug { eprintln!("✅ Decodificado con gzip: {} bytes", gzip_content.len()); }
        return Ok(gzip_content);
    }
    
    // Intentar detectar encoding y decodificar como texto plano
    let (cow, encoding_used, had_errors) = UTF_8.decode(bytes);
    if debug { 
        eprintln!("🔤 Encoding detectado: {:?}, errores: {}", encoding_used.name(), had_errors); 
    }
    
    // Si hay muchos errores, intentar con Latin1
    if had_errors {
        let (cow_latin, _, _) = WINDOWS_1252.decode(bytes);
        if debug { eprintln!("🔄 Probando con Windows-1252..."); }
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

fn resolver_javascript_challenge(html: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    // Buscar patrones comunes de challenge de Cloudflare
    if let Some(js_challenge) = extraer_javascript_challenge(html) {
        if debug { eprintln!("🔍 JavaScript encontrado: {}", &js_challenge[..js_challenge.len().min(100)]); }
        
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
        std::fs::write("/tmp/cloudflare_challenge.js", script_content)?;
        
        // Ejecutar con Node.js
        let output = std::process::Command::new("node")
            .arg("/tmp/cloudflare_challenge.js")
            .output()?;
        
        // Limpiar archivo temporal
        let _ = std::fs::remove_file("/tmp/cloudflare_challenge.js");
        
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        if debug { eprintln!("📤 Resultado Node.js: {}", result); }
        
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
    debug: bool
) -> Result<String, Box<dyn Error>> {
    if debug { eprintln!("📨 Enviando respuesta del challenge con token: {}...", &token[..token.len().min(50)]); }
    
    // Construir URL con el token extraído - usar __cf_chl_rt_tk que es el formato de redirect
    let url = if token.contains("__cf_chl_tk=") {
        // Convertir __cf_chl_tk a __cf_chl_rt_tk para redirect como hace el navegador
        let rt_token = token.replace("__cf_chl_tk=", "__cf_chl_rt_tk=");
        format!("https://dle.rae.es/{}?{}", urlencoding::encode(palabra), rt_token)
    } else if token.contains("__cf_chl_rt_tk=") {
        // Token de redirect - usar directamente
        format!("https://dle.rae.es/{}?{}", urlencoding::encode(palabra), token)
    } else if token.contains("__cf_chl_f_tk=") {
        // Token de form - construir URL específica
        format!("https://dle.rae.es/{}?{}", urlencoding::encode(palabra), token)
    } else {
        // Fallback - usar token como parámetro adicional
        format!("https://dle.rae.es/{}?m=form&challenge_token={}", urlencoding::encode(palabra), token)
    };
    
    if debug { eprintln!("🔗 URL con token: {}", url); }
    
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
    headers.insert("Sec-Ch-Ua", "\"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"".parse()?);
    headers.insert("Sec-Ch-Ua-Mobile", "?0".parse()?);
    headers.insert("Sec-Ch-Ua-Platform", "\"Linux\"".parse()?);
    headers.insert("Upgrade-Insecure-Requests", "1".parse()?);
    headers.insert("Priority", "u=0, i".parse()?);
    
    // Esperar un poco antes de enviar la respuesta (simular tiempo de carga)
    std::thread::sleep(std::time::Duration::from_millis(2000));
    
    let response = client
        .get(&url)
        .headers(headers)
        .send()?;
    
    if debug { eprintln!("📊 Respuesta post-challenge: Status {}", response.status()); }
    
    let bytes = response.bytes()?;
    let html_content = decodificar_contenido(&bytes, debug)?;
    
    if debug {
        std::fs::write("rae-debug-final.html", &html_content)?;
        eprintln!("🔍 HTML final guardado en rae-debug-final.html");
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

fn analizar_deteccion_cloudflare(palabra: &str, debug: bool) {
    println!("🔍 ANÁLISIS DE DETECCIÓN DE CLOUDFLARE");
    println!("=====================================");
    println!();
    
    let configuraciones = vec![
        ("Lynx Simulado", crear_cliente_lynx_simulado()),
        ("HTTP Moderno", crear_cliente_moderno()),
        ("HTTP Minimalista", crear_cliente_minimalista()),
        ("Chrome-like", crear_cliente_chrome()),
        ("Firefox-like", crear_cliente_firefox()),
        ("curl-like", crear_cliente_curl()),
    ];
    
    for (nombre, cliente_result) in configuraciones {
        println!("🧪 Probando configuración: {}", nombre);
        
        match cliente_result {
            Ok(cliente) => {
                match probar_configuracion(cliente, palabra, nombre, debug) {
                    Ok(status) => {
                        if status == 200 {
                            println!("✅ {} - ÉXITO (200 OK)", nombre);
                        } else {
                            println!("❌ {} - BLOQUEADO ({})", nombre, status);
                        }
                    }
                    Err(e) => {
                        println!("💥 {} - ERROR: {}", nombre, e);
                    }
                }
            }
            Err(e) => {
                println!("💥 {} - Error al crear cliente: {}", nombre, e);
            }
        }
        
        // Pausa entre requests para evitar rate limiting
        std::thread::sleep(std::time::Duration::from_millis(1000));
        println!();
    }
    
    println!("🔍 Análisis completado. Revisa los archivos debug-*.html para más detalles.");
}

fn crear_cliente_lynx_simulado() -> Result<reqwest::blocking::Client, Box<dyn Error>> {
    reqwest::blocking::Client::builder()
        .user_agent("Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3")
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .tcp_nodelay(false)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(1)
        .build()
        .map_err(|e| e.into())
}

fn crear_cliente_moderno() -> Result<reqwest::blocking::Client, Box<dyn Error>> {
    reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .http2_prior_knowledge()
        .build()
        .map_err(|e| e.into())
}

fn crear_cliente_minimalista() -> Result<reqwest::blocking::Client, Box<dyn Error>> {
    reqwest::blocking::Client::builder()
        .user_agent("MinimalClient/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.into())
}

fn crear_cliente_chrome() -> Result<reqwest::blocking::Client, Box<dyn Error>> {
    reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| e.into())
}

fn crear_cliente_firefox() -> Result<reqwest::blocking::Client, Box<dyn Error>> {
    reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/121.0")
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| e.into())
}

fn crear_cliente_curl() -> Result<reqwest::blocking::Client, Box<dyn Error>> {
    reqwest::blocking::Client::builder()
        .user_agent("curl/8.0.1")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.into())
}

fn probar_configuracion(
    cliente: reqwest::blocking::Client,
    palabra: &str,
    nombre: &str,
    debug: bool,
) -> Result<u16, Box<dyn Error>> {
    let url = format!("https://dle.rae.es/{}", urlencoding::encode(palabra));
    
    let mut headers = reqwest::header::HeaderMap::new();
    
    // Headers específicos según el tipo de cliente
    match nombre {
        "Lynx Simulado" => {
            headers.insert("Host", "dle.rae.es".parse()?);
            headers.insert("Accept", "text/html".parse()?);
        }
        "HTTP Minimalista" => {
            headers.insert("Accept", "*/*".parse()?);
        }
        "Chrome-like" => {
            headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8".parse()?);
            headers.insert("Accept-Language", "en-US,en;q=0.9".parse()?);
            headers.insert("Accept-Encoding", "gzip, deflate, br".parse()?);
            headers.insert("Sec-Fetch-Dest", "document".parse()?);
            headers.insert("Sec-Fetch-Mode", "navigate".parse()?);
            headers.insert("Sec-Fetch-Site", "none".parse()?);
            headers.insert("Sec-Fetch-User", "?1".parse()?);
        }
        "Firefox-like" => {
            headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".parse()?);
            headers.insert("Accept-Language", "en-US,en;q=0.5".parse()?);
            headers.insert("Accept-Encoding", "gzip, deflate, br".parse()?);
            headers.insert("Upgrade-Insecure-Requests", "1".parse()?);
        }
        "curl-like" => {
            headers.insert("Accept", "*/*".parse()?);
        }
        _ => {
            headers.insert("Accept", "text/html".parse()?);
            headers.insert("Accept-Language", "es,en;q=0.9".parse()?);
            headers.insert("Accept-Encoding", "gzip".parse()?);
        }
    }
    
    if debug {
        println!("  🌐 URL: {}", url);
        println!("  📋 Headers: {:#?}", headers);
    }
    
    let response = cliente.get(&url).headers(headers).send()?;
    let status_code = response.status().as_u16();
    
    if debug {
        println!("  📊 Status: {}", status_code);
        println!("  📊 Response headers: {:#?}", response.headers());
        
        if let Some(cf_ray) = response.headers().get("cf-ray") {
            println!("  ☁️  Cloudflare Ray ID: {:?}", cf_ray);
        }
    }
    
    // Guardar HTML para análisis
    let bytes = response.bytes()?;
    let html_content = String::from_utf8_lossy(&bytes);
    let filename = format!("debug-{}.html", nombre.to_lowercase().replace(' ', "-"));
    std::fs::write(&filename, html_content.as_bytes())?;
    
    if debug {
        println!("  🔍 HTML guardado en {}", filename);
    }
    
    Ok(status_code)
}
