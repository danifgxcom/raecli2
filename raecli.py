#!/usr/bin/env python3
import cloudscraper
import re
import sys

# Verifica si se proporcionó una palabra
if len(sys.argv) != 2:
    print("Uso: python3 rae.py <palabra>")
    print("Ejemplo: python3 rae.py programar")
    sys.exit(1)

# Palabra a buscar
palabra = sys.argv[1]
url = f"https://dle.rae.es/{palabra}"

# Crea un scraper que maneje Cloudflare
scraper = cloudscraper.create_scraper()

try:
    # Obtiene la página
    response = scraper.get(url).text
    # Busca la definición en el HTML
    match = re.search(r'<p>\d+\.\s+[a-z]+\.\s+([^<]+)', response)
    if match:
        print(f"Definición de '{palabra}': {match.group(1)}")
    else:
        print("No se encontró la definición. Puede que la palabra no exista o el formato haya cambiado.")
        print(f"Intenta abrir la URL en un navegador: {url}")
except Exception as e:
    print(f"Error al obtener la página: {e}")
    print(f"Intenta abrir la URL en un navegador: {url}")