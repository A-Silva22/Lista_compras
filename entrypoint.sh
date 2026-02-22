#!/bin/sh
python manage.py collectstatic --noinput
exec gunicorn lista_compras.wsgi:application --bind 0.0.0.0:8000 --workers 3
