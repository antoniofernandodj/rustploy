Boa pergunta — e a resposta tem uma reviravolta interessante no fim. Sem a glacier, você monta isso com três peças e o iced daemon. Vou por partes.

O princípio

O iced não tem suporte nativo a tray, e ele é dono do event-loop do winit — você não injeta a bandeja nesse loop. Então o padrão é:

1. A bandeja roda numa thread própria (com o loop de eventos do SO dela).
2. Uma Subscription faz a ponte dos eventos da bandeja de volta pro seu update.
3. Você usa iced::daemon (não application) pra controlar o ciclo de vida das janelas — e simplesmente não chama iced::exit() quando a última fecha.

1. Dependências

```toml
[dependencies]
iced = { version = "0.14", features = ["tokio"] }
tray-icon = "0.24"

[target.'cfg(target_os = "linux")'.dependencies]
gtk = "0.18"   # pra inicializar e rodar o loop GT
```

2. A thread da bandeja

tray-icon cria a bandeja e precisa de um loop de e— GTK no Linux, message-pump Win32 no Windows. Como o iced é dono da thread principal, a bandeja vai pra uma thread separada:

```rust
use tray_icon::{TrayIconBuilder, Icon,
    menu::{Menu, MenuItem, PredefinedMenuItem}};

fn spawn_tray() {
    std::thread::spawn(|| {
        #[cfg(target_os = "linux")]
        gtk::init().unwrap();

        let menu = Menu::new();
        menu.append(&MenuItem::with_id("open", "Op
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&MenuItem::with_id("quit", "Qu

        // _tray precisa VIVER enquanto a thread e
        let _tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(load_icon())            //
            .with_tooltip("My App")
            .with_menu_on_left_click(cfg!(target_o
            .build()
            .unwrap();

        #[cfg(target_os = "linux")]
        gtk::main();   // bloqueia rodando o loop
        // no Windows: um loop PeekMessage/Dispatcn()
    });
}
```

3. A ponte: subscription lendo os canais globais

O tray-icon publica cliques num canal global (Menuam sync). Você faz polling dele num stream e
transforma numa Subscription:

```rust
use futures::{SinkExt, Stream};

fn tray_subscription() -> iced::Subscription<Messa
    iced::Subscription::run(tray_events)   // a chave vem do tipo do fn
}

fn tray_events() -> impl Stream<Item = Message> {
    iced::stream::channel(16, |mut out| async move
        use tray_icon::menu::MenuEvent;
        loop {
            while let Ok(ev) = MenuEvent::receiver
                let _ = out.send(Message::Tray(ev.id.0)).await;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
}
```

4. O app daemon — ciclo de vida

```rust
use std::collections::HashSet;
use iced::{window, Task, Subscription, Element};

#[derive(Default)]
struct App {
    windows: HashSet<window::Id>,
    main: Option<window::Id>,
    // aqui vive TODO o seu estado: login, dados, etc.
}

#[derive(Debug, Clone)]
enum Message {
    Tray(String),
    Opened(window::Id),
    Closed(window::Id),
}

fn main() -> iced::Result {
    iced::daemon(boot, App::update, App::view)
        .subscription(App::subscription)
        .title(|_, _| "My App".into())
        .run()
}

fn boot() -> (App, Task<Message>) {
    spawn_tray();
    let (id, open) = window::open(window::Settings::default());
    let mut app = App::default();
    app.main = Some(id);
    app.windows.insert(id);
    (app, open.map(Message::Opened))
}

impl App {
    fn update(&mut self, msg: Message) -> Task<Mes
        match msg {
            Message::Tray(id) => match id.as_str()
                "open" => match self.main.filter(|id| self.windows.contains(id)) {
                    Some(id) => window::gain_focusfoca
                    None => {                                    // fechada: reabre
                        let (id, open) = window::o());
                        self.main = Some(id);
                        self.windows.insert(id);
                        open.map(Message::Opened)
                    }
                },
                "quit" => iced::exit(),                          // <- ÚNICA saída
                _ => Task::none(),
            },
            Message::Opened(id) => { self.windows.
            Message::Closed(id) => {
                self.windows.remove(&id);
                // A CHAVE: NÃO chamar iced::exit() aqui, mesmo com zero janelas.
                // O app segue vivo em background, representado pela bandeja.
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::close_events().map(Message::Closed),
            tray_subscription(),
            // aqui também moram seus streams (SSE/WebSocket) — ver abaixo
        ])
    }

    fn view(&self, _id: window::Id) -> Element<Mes
}
```

A reviravolta: "manter o login" é de graça no iced

Aqui está a parte importante. Todo aquele trabalho de "motor headless" que fiz na glacier não existe num app iced escrito à mão —
e o motivo é arquitetural:

- Na glacier, cada janela é um motor independente os). Fechar a janela destruía o motor → perdialogin e SSE. Por isso precisei reter o motor sem janela.
- No iced puro com daemon, seu estado é um App só,enhuma janela. Fechar uma janela não toca no App —o login, os dados e as subscriptions continuam exatamente onde estavam. As janelas são só "vistas" descartáveis sobre esse estado.

Então, no iced puro:
- login preservado: automático — ele mora no App,
- SSE vivo com a janela fechada: automático — o stn(), que o iced continua rodando enquanto o App
existe, independente de haver janela;
- notificações de deploy: seu handler do SSE dispara notify-rust normalmente, com ou sem janela.

Ou seja: a complexidade do "headless" foi um imposto da abstração da glacier (um motor por janela). Escrevendo direto no iced,
você só precisa de: não chamar exit() no último Cle sozinho.

Pegadinhas de plataforma (as que doeram)

- Linux: o TrayIconEvent de clique no ícone não é  por isso o with_menu_on_left_click(true) no Linux
(o clique abre o menu). No Windows o clique esquerbrir a janela direto.
- Wayland: não dá pra "esconder" (set_visible é no — pra janela sumir você destrói e reabre depois.Como o estado vive no App, destruir a janela não custa nada aqui.
- macOS: a bandeja exige a thread principal — a thsso conflita com o iced ser dono da main thread, eé a razão de eu ter deixado macOS de fora.
- Atualizar o menu (ex.: alternar "Disable/Enable muda são !Send e vivem na thread da bandeja, entãovocê manda um comando por canal pra thread aplicar item.set_text(...) lá — não dá pra mexer neles do lado do iced.

Se quiser, monto isso como um exemplo compilável dparte.

✻ Worked for 1m 24s

※ recap: Goal: add a system-tray to the Rustploy GUIthe app alive with login and deploy notificationsintact. That's built, published, tested, and pusheo do it in pure iced. (disable recaps in /config)
