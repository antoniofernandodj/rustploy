// screens/login.js — tela de login. A maior parte do estado (url/token/
// remember/erro) já mora no Alpine.store('app') (app.js) porque a topbar e o
// fluxo de reconexão também precisam dele; este componente só encapsula o
// submit do formulário.
document.addEventListener("alpine:init", () => {
  Alpine.data("login", () => ({
    get store() {
      return Alpine.store("app");
    },
    submit() {
      this.store.connect();
    },
  }));
});
