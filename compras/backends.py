import hashlib
import logging
from django.contrib.auth.backends import BaseBackend
from django.contrib.auth import get_user_model

User = get_user_model()
logger = logging.getLogger(__name__)


class HashedPasswordBackend(BaseBackend):
    """Dual-check backend during password-hashing migration.

    Accepts plain `password` (new path) and falls back to legacy SHA-256
    forms. When a legacy match wins, the row is rehashed with PBKDF2 over
    the plaintext so the next login takes the fast path.

    Remove the legacy branches once telemetry shows zero hits.
    """

    def authenticate(self, request, username=None, password=None,
                     password_hash=None, **kwargs):
        if not username:
            return None
        try:
            user = User.objects.get(username=username)
        except User.DoesNotExist:
            return None

        # 1. Modern path: stored hash is pbkdf2(plain), form sent plain.
        if password and user.check_password(password):
            return user

        # 2. Legacy template still cached: form sent sha256 hex as
        #    `password_hash`, stored hash is pbkdf2(sha256(plain)).
        if password_hash and user.check_password(password_hash):
            logger.info('legacy_auth_path=template username=%s', username)
            return user

        # 3. Migration path: form sent plain, stored hash is pbkdf2(sha256(plain)).
        #    Verify via the legacy SHA-256 derivation, then rehash.
        if password:
            legacy = hashlib.sha256(password.encode()).hexdigest()
            if user.check_password(legacy):
                logger.info('legacy_auth_path=migrate username=%s', username)
                user.set_password(password)
                user.save(update_fields=['password'])
                return user

        return None

    def get_user(self, user_id):
        try:
            return User.objects.get(pk=user_id)
        except User.DoesNotExist:
            return None
