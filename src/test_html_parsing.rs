use scraper::{Html, Selector};
use std::error::Error;
use std::fs;

// Import the functions from main.rs
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

fn main() -> Result<(), Box<dyn Error>> {
    // Leer el archivo HTML
    let html_content = fs::read_to_string("debug-lynx-con-tls-fingerprint.html")?;

    // Intentar extraer definiciones con el método principal
    match extraer_definiciones_html(&html_content) {
        Ok(definiciones) => {
            println!("=== DEFINICIONES EXTRAÍDAS CON extraer_definiciones_html ===");
            println!("{}", definiciones);
        }
        Err(e) => {
            println!(
                "Error al extraer definiciones con extraer_definiciones_html: {}",
                e
            );

            // Intentar con el método de respaldo
            println!("\n=== DEFINICIONES EXTRAÍDAS CON filtrar_contenido_html ===");
            println!("{}", filtrar_contenido_html(&html_content));
        }
    }

    Ok(())
}
