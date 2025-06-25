# Implementación de TLS Fingerprint de Lynx en Rust

Este documento describe la implementación de un cliente TLS personalizado en Rust que intenta imitar el fingerprint TLS de Lynx para evadir la detección de Cloudflare.

## Resumen

Cloudflare utiliza técnicas avanzadas de detección de TLS fingerprinting para identificar y bloquear clientes HTTP automatizados. El navegador Lynx, por alguna razón, es permitido por Cloudflare para acceder a sitios como rae.es, mientras que otros clientes HTTP son bloqueados.

Hemos implementado un cliente TLS personalizado en Rust que intenta imitar el fingerprint TLS de Lynx, utilizando la biblioteca rustls con configuración personalizada.

## Enfoque Técnico

### 1. Análisis del TLS Fingerprint de Lynx

Lynx utiliza la biblioteca GNUTLS para su implementación TLS. A través del análisis del código fuente de Lynx y capturas de tráfico (tcpdump), identificamos las siguientes características clave:

- Usa TLS 1.2 como protocolo preferido
- Tiene un conjunto específico de cipher suites
- Envía headers HTTP minimalistas
- Tiene un comportamiento de red distintivo (timing, TCP/IP)

### 2. Implementación en Rust

Nuestra implementación utiliza:

- **rustls**: Una biblioteca TLS en Rust puro
- **webpki**: Para verificación de certificados
- **webpki-roots**: Para raíces de confianza

La implementación personalizada incluye:

```rust
fn hacer_peticion_tls_personalizada(url: &str, debug: bool) -> Result<String, Box<dyn Error>> {
    // Configuración personalizada de TLS
    let mut config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    
    // Personalizar cipher suites y versiones de protocolo
    unsafe {
        let mut cipher_suites = Vec::new();
        cipher_suites.push(rustls::cipher_suite::TLS13_AES_128_GCM_SHA256);
        // ... más cipher suites ...
        
        config.dangerous().set_cipher_suites(cipher_suites);
        
        let versions = vec![rustls::ProtocolVersion::TLSv1_2];
        config.dangerous().set_protocol_versions(&versions)?;
    }
    
    // Simular el timing de Lynx
    std::thread::sleep(std::time::Duration::from_millis(150));
    
    // Implementación del cliente HTTP básico
    // ...
}
```

### 3. Integración con el Proyecto

La implementación se integra con el proyecto existente a través de:

- Una variable de entorno `USE_CUSTOM_TLS` para activar la implementación personalizada
- Modificación de la función `simular_lynx_exacto()` para usar la implementación personalizada
- Adición de un test en `analizar_deteccion_cloudflare()` para probar la implementación

## Resultados y Limitaciones

### Resultados

- La implementación personalizada proporciona un mayor control sobre el handshake TLS
- Permite configurar cipher suites y versiones de protocolo específicas
- Simula el comportamiento de red de Lynx

### Limitaciones

1. **Compatibilidad de Dependencias**: Las bibliotecas GNUTLS para Rust tienen problemas de compatibilidad
2. **Precisión del Fingerprint**: Es extremadamente difícil replicar exactamente el fingerprint TLS de Lynx
3. **Evolución de Cloudflare**: Cloudflare actualiza constantemente sus métodos de detección

## Conclusiones y Recomendaciones

### Conclusiones

1. Es técnicamente posible implementar un cliente TLS personalizado en Rust que imite algunas características del fingerprint de Lynx
2. Sin embargo, la implementación completa y precisa es extremadamente compleja
3. La estrategia actual de fallback a Lynx real sigue siendo la más confiable (el fallback ha sido eliminado intencionadamente para garantizar que el código TLS funciona)

## Uso

Para activar la implementación TLS personalizada:

```bash
USE_CUSTOM_TLS=1 ./target/debug/raecli2 palabra --http
```

Para analizar la efectividad:

```bash
USE_CUSTOM_TLS=1 ./target/debug/raecli2 palabra --analizar
```

## Referencias

1. Código fuente de Lynx: https://lynx.invisible-island.net/current/
2. Documentación de rustls: https://docs.rs/rustls/
3. JA3 Fingerprinting: https://github.com/salesforce/ja3