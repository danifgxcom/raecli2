#!/bin/bash

# Agregar todos los archivos al staging
git add .

# Hacer commit con mensaje descriptivo
git commit -m "Versión funcionando con GIF básico - antes de mejoras visuales"

# Crear tag
git tag -a funcionando -m "Tag de versión funcionando con GIF básico"

echo "✅ Commit realizado y tag 'funcionando' creado"
echo "Para ver los commits: git log --oneline"
echo "Para ver los tags: git tag -l"
