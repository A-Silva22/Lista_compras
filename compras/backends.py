import hashlib
from django.contrib.auth.backends import BaseBackend
from django.contrib.auth import get_user_model

User = get_user_model()


class HashedPasswordBackend(BaseBackend):
    """
    Authenticates using a client-side SHA-256 hashed password.
    The server never receives the plaintext password — only the hash.
    The hash is then verified against the stored (double-hashed) value.
    """

    def authenticate(self, request, username=None, password_hash=None, password=None, **kwargs):
        if username is None:
            return None
        # Support both: client-side hashed password and plain password (e.g. Django admin)
        secret = password_hash or password
        if secret is None:
            return None
        try:
            user = User.objects.get(username=username)
        except User.DoesNotExist:
            return None
        if user.check_password(secret):
            return user
        # If plain password was provided (e.g. Django admin), try SHA-256 hashing it
        if password and not password_hash:
            hashed = hashlib.sha256(password.encode()).hexdigest()
            if user.check_password(hashed):
                return user
        return None

    def get_user(self, user_id):
        try:
            return User.objects.get(pk=user_id)
        except User.DoesNotExist:
            return None
