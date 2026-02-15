from django.contrib.auth.backends import BaseBackend
from django.contrib.auth import get_user_model

User = get_user_model()


class HashedPasswordBackend(BaseBackend):
    """
    Authenticates using a client-side SHA-256 hashed password.
    The server never receives the plaintext password — only the hash.
    The hash is then verified against the stored (double-hashed) value.
    """

    def authenticate(self, request, username=None, password_hash=None, **kwargs):
        if username is None or password_hash is None:
            return None
        try:
            user = User.objects.get(username=username)
        except User.DoesNotExist:
            return None
        if user.check_password(password_hash):
            return user
        return None

    def get_user(self, user_id):
        try:
            return User.objects.get(pk=user_id)
        except User.DoesNotExist:
            return None
