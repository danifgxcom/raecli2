#!/bin/bash

# Verifica si se ha proporcionado una palabra
if [ -z "$1" ]; then
    echo "Uso: $0 <palabra>"
    echo "Ejemplo: $0 programar"
    exit 1
fi

# Palabra a buscar
PALABRA="$1"
URL="https://dle.rae.es/$PALABRA"

# Verifica si wkhtmltopdf y curl están instalados
if ! command -v curl &> /dev/null; then
    echo "Error: curl no está instalado. Instálalo con: sudo apt install curl"
    exit 1
fi
if ! command -v wkhtmltopdf &> /dev/null; then
    echo "Error: wkhtmltopdf no está instalado. Instálalo con: sudo apt install wkhtmltopdf"
    exit 1
fi

# Archivos temporales
COOKIE_FILE=$(mktemp)
RESPONSE_FILE=$(mktemp)

# Usa wkhtmltopdf para renderizar la página y obtener el HTML
wkhtmltopdf --enable-javascript --javascript-delay 5000 "$URL" /dev/null 2>&1 | \
    curl -s -L \
    -b "$COOKIE_FILE" \
    -c "$COOKIE_FILE" \
    -H "User-Agent: Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:129.0) Gecko/20100101 Firefox/129.0" \
    -H "Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8" \
    -H "Accept-Language: es-ES,es;q=0.8,en-US;q=0.5,en;q=0.3" \
    -H "Accept-Encoding: gzip, deflate, br" \
    -H "Connection: keep-alive" \
    -H "Upgrade-Insecure-Requests: 1" \
    -H "Sec-Fetch-Dest: document" \
    -H "Sec-Fetch-Mode: navigate" \
    -H "Sec-Fetch-Site: none" \
    -H "Sec-Fetch-User: ?1" \
    -H "DNT: 1" \
    --compressed \
    "$URL" > "$RESPONSE_FILE"

# Verifica si la respuesta contiene un bloqueo de Cloudflare
if grep -qi "cloudflare" "$RESPONSE_FILE" || grep -qi "Just a moment" "$RESPONSE_FILE"; then
    echo "Error: Cloudflare sigue bloqueando la solicitud."
    echo "Contenido de la respuesta (primeras 20 líneas):"
    head -n 20 "$RESPONSE_FILE"
    echo "Intenta abrir la URL en un navegador: $URL"
    rm -f "$COOKIE_FILE" "$RESPONSE_FILE"
    exit 1
fi

# Extrae la definición
DEFINITION=$(cat "$RESPONSE_FILE" | \
    grep -A 10 "<p>${PALABRA}" | \
    sed -n 's/.*<p>\([0-9]\.\s*[a-z]\+\.\s*[^<]*\).*/\1/p' | \
    head -n 1)

# Verifica si se obtuvo una definición
if [ -z "$DEFINITION" ]; then
    echo "No se pudo extraer la definición. Puede que la palabra no exista o el formato de la página haya cambiado."
    echo "Contenido de la respuesta (primeras 20 líneas):"
    head -n 20 "$RESPONSE_FILE"
    echo "Intenta abrir la URL en un navegador: $URL"
else
    echo "$DEFINITION"
fi

# Limpia archivos temporales
rm -f "$COOKIE_FILE" "$RESPONSE_FILE"