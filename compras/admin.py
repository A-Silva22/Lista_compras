import hashlib
from django.contrib import admin
from django.contrib.auth.admin import UserAdmin
from django.contrib.auth.forms import AdminPasswordChangeForm, UserCreationForm
from django.contrib.auth import get_user_model
from .models import Lista, ListaPartilha, LinkPartilha, Artigo

User = get_user_model()


class SHA256AdminPasswordChangeForm(AdminPasswordChangeForm):
    """Pre-hash password with SHA-256 before storing, matching client-side login."""
    def save(self, commit=True):
        password = self.cleaned_data["password1"]
        password_hash = hashlib.sha256(password.encode()).hexdigest()
        self.user.set_password(password_hash)
        if commit:
            self.user.save()
        return self.user


class SHA256UserCreationForm(UserCreationForm):
    """Pre-hash password with SHA-256 when creating users via admin."""
    def save(self, commit=True):
        user = super().save(commit=False)
        password = self.cleaned_data["password1"]
        password_hash = hashlib.sha256(password.encode()).hexdigest()
        user.set_password(password_hash)
        if commit:
            user.save()
        return user


class SHA256UserAdmin(UserAdmin):
    change_password_form = SHA256AdminPasswordChangeForm
    add_form = SHA256UserCreationForm


admin.site.unregister(User)
admin.site.register(User, SHA256UserAdmin)

admin.site.register(Lista)
admin.site.register(ListaPartilha)
admin.site.register(LinkPartilha)
admin.site.register(Artigo)
