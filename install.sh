#!/bin/bash
echo "🔧 Instalando RAE CLI..."
if [ ! -f "target/release/raecli2" ]; then
    echo "❌ Binario no encontrado. Ejecuta primero ./build.sh"
    exit 1
fi
sudo cp target/release/raecli2 /usr/local/bin/
echo "✅ RAE CLI instalado correctamente en /usr/local/bin/raecli2"
echo "Ahora puedes usar: raecli2 <palabra>"
