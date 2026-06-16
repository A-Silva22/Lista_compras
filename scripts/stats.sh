#!/usr/bin/env bash
# ListaIsto — relatório de atividade do site.
# Lê a base de dados de PRODUÇÃO através de SSH (host "lc") + docker, sem expor
# a BD nem as credenciais (lidas do .env no servidor).
#
#   Uso:   ./scripts/stats.sh
#   Host:  por omissão "lc" (definido no ~/.ssh/config). Override: LISTAISTO_SSH=outro ./scripts/stats.sh
set -euo pipefail
HOST="${LISTAISTO_SSH:-lc}"

ssh -o BatchMode=yes -o ConnectTimeout=15 "$HOST" 'bash -s' <<'REMOTE'
set -euo pipefail
cd /root/Lista_compras
ROOT=$(grep -E '^DB_ROOT_PASSWORD=' .env | cut -d= -f2-)
DBN=$(grep -E '^DB_NAME=' .env | cut -d= -f2-)
q() { docker exec lista_compras-db-1 mariadb -u root -p"$ROOT" "$DBN" -N -B -e "$1" 2>/dev/null; }

echo "====================================================="
echo "  ListaIsto — atividade   ($(date '+%Y-%m-%d %H:%M'))"
echo "====================================================="
printf "Contas (total) ............ %s\n" "$(q 'SELECT COUNT(*) FROM auth_user')"
printf "  com email associado ..... %s\n" "$(q 'SELECT COUNT(*) FROM user_email')"
echo
echo "-- Registos novos --"
printf "  últimas 24h ............. %s\n" "$(q 'SELECT COUNT(*) FROM auth_user WHERE date_joined >= NOW() - INTERVAL 1 DAY')"
printf "  últimos 7 dias .......... %s\n" "$(q 'SELECT COUNT(*) FROM auth_user WHERE date_joined >= NOW() - INTERVAL 7 DAY')"
printf "  últimos 30 dias ......... %s\n" "$(q 'SELECT COUNT(*) FROM auth_user WHERE date_joined >= NOW() - INTERVAL 30 DAY')"
echo
echo "-- Ativos (com login recente) --"
printf "  últimas 24h ............. %s\n" "$(q 'SELECT COUNT(*) FROM auth_user WHERE last_login >= NOW() - INTERVAL 1 DAY')"
printf "  últimos 7 dias .......... %s\n" "$(q 'SELECT COUNT(*) FROM auth_user WHERE last_login >= NOW() - INTERVAL 7 DAY')"
printf "  últimos 30 dias ......... %s\n" "$(q 'SELECT COUNT(*) FROM auth_user WHERE last_login >= NOW() - INTERVAL 30 DAY')"
echo
echo "-- Conteúdo --"
printf "Listas .................... %s\n" "$(q 'SELECT COUNT(*) FROM compras_lista')"
printf "Artigos ................... %s\n" "$(q 'SELECT COUNT(*) FROM compras_artigo')"
printf "Listas partilhadas ........ %s\n" "$(q 'SELECT COUNT(DISTINCT lista_id) FROM compras_listapartilha')"
printf "Links de partilha ativos .. %s\n" "$(q 'SELECT COUNT(*) FROM compras_linkpartilha WHERE expira_em > NOW()')"
echo
echo "-- Registos por dia (últimos 7 dias) --"
q "SELECT DATE(date_joined) AS dia, COUNT(*) AS novos FROM auth_user WHERE date_joined >= NOW() - INTERVAL 7 DAY GROUP BY DATE(date_joined) ORDER BY dia" \
  | awk 'BEGIN{FS="\t"} {printf "  %s : %s\n", $1, $2}'
echo
echo "-- Últimos 5 registos --"
q "SELECT username, DATE_FORMAT(date_joined,'%Y-%m-%d %H:%i') FROM auth_user ORDER BY id DESC LIMIT 5" \
  | awk 'BEGIN{FS="\t"} {printf "  %-20s %s\n", $1, $2}'
echo "====================================================="
echo "Nota: \"Ativos\" = login recente. Quem fica com sessão"
echo "iniciada (sem voltar a entrar) não conta — ver README."
REMOTE
