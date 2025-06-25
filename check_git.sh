#!/bin/bash

echo "📊 Estado del repositorio Git:"
echo "================================"

echo -e "\n🔍 Git status:"
git status --short

echo -e "\n📝 Últimos commits:"
git log --oneline -n 5

echo -e "\n🏷️  Tags existentes:"
git tag -l

echo -e "\n🌿 Ramas:"
git branch
