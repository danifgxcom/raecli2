# RAE CLI

Una herramienta de línea de comandos en Rust para buscar definiciones de palabras en el Diccionario de la RAE.

## ¿Por qué este proyecto?

Cloudflare protege la página de la RAE y bloquea herramientas como curl o reqwest. Este proyecto usa TLS fingerprinting del navegador Lynx para acceder sin problemas.

## Arquitectura Técnica

### Cliente TLS Personalizado

Usa rustls y TcpStream para crear peticiones HTTP que imitan exactamente el navegador Lynx:

- Headers HTTP exactos de Lynx
- User-Agent: `Lynx/2.9.0 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3`
- Cipher suites y configuración TLS de GNUTLS
- Timing de conexión similar al comportamiento real

Los parámetros TLS se obtuvieron analizando el tráfico real de Lynx con tcpdump:
- Cipher Suite: `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`
- Extensiones TLS básicas (SNI, Supported Groups)
- Algoritmo ECDHE con curva secp256r1

### ¿Por qué Lynx?

Lynx es más fácil de imitar que navegadores modernos:

- **TLS simple**: Solo unos pocos cipher suites vs 20+ en Chrome
- **Headers mínimos**: 4 headers vs 15+ en navegadores modernos
- **Sin JavaScript**: Lynx es un navegador de texto, no ejecuta JS
- **Consistente**: Se comporta igual en todos los sistemas

Complejidad de imitar diferentes navegadores:

- **Lynx**: 4 headers, cipher suites fijos
- **Chrome**: 15+ headers dinámicos, 20+ cipher suites
- **Firefox**: 12+ headers, diferentes por OS

Cloudflare bloquea herramientas obvias (curl, wget, requests) pero permite navegadores legítimos como Lynx.

Comparación de fingerprints TLS:

```
Lynx:    769,47-53-5-10,0-10-11,23-24,0
Chrome:  771,4865-4866-4867-...(50+ valores más)
```

Lynx es mucho más simple de replicar.



### Cómo funciona

1. **Imita Lynx exactamente**:
   - User-Agent de Lynx 2.9.0
   - Headers HTTP mínimos
   - Configuración TLS de GNUTLS
   - Timing de conexión natural

2. **Extrae información TLS real**:
   - Protocolo TLS negociado
   - Cipher suite utilizado
   - Certificados del servidor


## Requisitos

- Rust 1.60 o superior
- Conexión a Internet

No necesita dependencias externas. Lynx no ejecuta JavaScript, así que Cloudflare no pide verificaciones adicionales.

## Instalación

### Instalación

```bash
git clone https://github.com/tu-usuario/raecli2.git
cd raecli2
cargo build --release
./install.sh  # copia a /usr/local/bin
```

## Uso

### Uso básico

```bash
raecli2 casa
raecli2 perro --debug    # con información técnica
```

El modo `--debug` genera archivos de log con detalles del handshake TLS y guarda el HTML de respuesta.

## Detalles técnicos

La función principal establece una conexión TCP directa, configura rustls para imitar GNUTLS, envía headers exactos de Lynx y extrae las definiciones del HTML.

Para extraer definiciones usa múltiples selectores CSS y valida que el contenido sean definiciones reales (no elementos de navegación).

## Solución de problemas

Si no funciona:
1. Usa `--debug` para ver los logs
2. Verifica tu conexión a Internet

Cloudflare puede actualizar su detección ocasionalmente.

## Licencia

MIT

## Estructura del código

```
src/
├── main.rs                 # Binario principal
├── lib.rs                  # Biblioteca
├── client/                 # Clientes HTTP
├── parser/                 # Extracción de definiciones
├── types/                  # Tipos de datos
└── utils/                  # Utilidades
```

El código separa claramente responsabilidades: clientes obtienen contenido, parsers extraen definiciones, validadores verifican el contenido.

Las definiciones se validan buscando categorías gramaticales (f., m., adj.) y filtrando elementos de navegación del sitio.


## Tests

Incluye 67 tests que cubren:
- Extracción de definiciones
- Validación de contenido
- Manejo de errores
- Casos límite (HTML malformado, definiciones vacías, etc.)

Para uso educativo. Respeta los términos de servicio de los sitios web.