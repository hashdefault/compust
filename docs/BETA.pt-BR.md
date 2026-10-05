# Teste da beta

[English (US)](BETA.md) | [Português (Brasil)](BETA.pt-BR.md)

O Compust 0.3.0-beta.2 é a quinta beta, voltada a testes controlados da comunidade. Ele ainda não substitui o picom. O suporte é declarado somente onde há evidências registradas; outras configurações são bem-vindas para testes, mas continuam sem validação.

Esta beta traz os recursos da primeira etapa do marco 1.0, sombras, suspensão da composição em tela cheia e regras que escolhem janelas pelo foco, junto com cantos arredondados e recarga da configuração quando o arquivo é salvo, e corrige dois erros de renderização. As [notas da versão](releases/v0.3.0-beta.2.md) listam as mudanças.

## Escopo declarado

Uma máquina registrada rodou o código desta beta: um desktop AMD com gráficos Radeon Vega integrados, no Xorg.

| Área | Registrado com o código desta beta | Evidências |
| --- | --- | --- |
| Servidor X | Xorg 21.1.24 em sessão nativa | [Sessões de desktop](DESKTOP_TESTING.pt-BR.md#sessões-xorg-registradas-no-desktop-com-radeon-vega-2026-10-05) |
| Gerenciador de janelas | i3 4.25.1 | Mesmas sessões |
| GPU e driver | AMD Ryzen 5 5600GT com gráficos Radeon Vega e radeonsi, o driver modesetting e glamor sem TearFree | Mesmas sessões |
| Monitores | Uma saída em 1920×1080 a 60 Hz | Mesmas sessões |
| Renderização | Present e XRender direto, fades, transparência e desfoque | Mesmas sessões |
| Recursos | Sombras, regras pelo foco, cantos arredondados e suspensão em tela cheia, no XRender e no renderizador de GPU, sem gerenciador de janelas | [Verificações de recursos](DESKTOP_TESTING.pt-BR.md#cantos-arredondados-registrados-no-desktop-com-radeon-vega-2026-10-05) |

As sessões de desktop rodaram em `ed2cdb0`, antes dos cantos arredondados, que vêm desligados, e antes de uma correção no renderizador de GPU, que elas não usaram. As verificações de recursos rodaram em `1fc0a1c`, cujo código do compositor difere do desta versão apenas no número da versão. Ambas usaram compilações locais com janelas sintéticas; nenhuma foi repetida com o binário empacotado. O hotplug físico e a suspensão e retomada não foram registrados no Xorg.

O desktop com RX 9060 XT e XLibre rodou a 0.3.0-beta.1 com Xmonad, Openbox, i3 e bspwm, e registrou os testes de sombras, foco e tela cheia nos dois renderizadores antes desta beta; o laptop Intel rodou as betas 0.2.0. Seus registros, listados na [matriz de compatibilidade](ROADMAP.pt-BR.md#matriz-de-compatibilidade), não cobrem o código desta beta. GPUs NVIDIA, GPUs Intel e o XLibre com esta beta, outros gerenciadores de janelas, taxas de atualização claramente diferentes, mais de dois monitores e HDR não foram testados. Relatos de qualquer uma dessas configurações são especialmente úteis.

## Instalar

No Debian, Ubuntu, Linux Mint, Fedora e openSUSE Tumbleweed, é possível instalar pacotes, como o [README](../README.pt-BR.md#instalação) descreve. Caso contrário, baixe `compust-0.3.0-beta.2-x86_64-linux.tar.gz` e `SHA256SUMS` na [página da versão](https://github.com/hashdefault/compust/releases/tag/v0.3.0-beta.2), depois verifique e extraia os arquivos:

```sh
sha256sum -c SHA256SUMS
tar -xzf compust-0.3.0-beta.2-x86_64-linux.tar.gz
cd compust-0.3.0-beta.2-x86_64-linux
./compust --version
```

O binário exige Linux x86_64 com glibc 2.34 ou mais recente. `BUILDINFO` registra a revisão do código, o compilador e o comando de compilação. Para compilar a partir do código-fonte, instale a toolchain Rust fixada pelo projeto e execute:

```sh
git clone --branch v0.3.0-beta.2 https://github.com/hashdefault/compust.git
cd compust
cargo build --release --locked
```

A compilação gera `target/release/compust`. Copie qualquer um dos binários para um diretório do seu `PATH`, como `~/.local/bin`.

## Executar

Verifique o servidor antes. `--diagnose` lista as extensões usadas pelo Compust e pode rodar com segurança ao lado de outro compositor:

```sh
compust --diagnose
```

Copie a configuração de exemplo, ajuste se quiser e valide:

```sh
mkdir -p ~/.config/compust
cp compust.example.toml ~/.config/compust/compust.toml
compust --check-config
```

Sem `--config`, o Compust lê `compust/compust.toml` em `$XDG_CONFIG_HOME`, por padrão `~/.config`, e `--check-config` informa o arquivo que encontrou. Pare o compositor atual e inicie o Compust em um terminal da mesma sessão X:

```sh
pkill -x picom
compust
```

Se outro compositor ainda controlar a tela, o Compust se recusa a iniciar e informa o motivo. Para iniciá-lo com o Xmonad, substitua a linha de inicialização do seu compositor, como `spawnOnce "picom ..."`, pela linha a seguir:

```haskell
spawnOnce "compust"
```

Com o i3, use `exec --no-startup-id compust` na configuração dele. Com `~/.xinitrc`, inicie `compust &` antes do gerenciador de janelas.

Salvar a configuração a aplica sem reiniciar, depois que o arquivo fica um décimo de segundo sem mudar. `pkill -USR1 -x compust` também a recarrega e lê um diretório de configuração criado depois que o Compust iniciou. Um arquivo que não puder ser lido ou for inválido é rejeitado com um aviso, e as opções em uso continuam valendo. O [README](../README.pt-BR.md#configuração) descreve todas as opções e as [regras por janela](../README.pt-BR.md#regras-por-janela).

### Experimentar o renderizador de GPU

`backend = "gl"` na configuração desenha com OpenGL ES na GPU do servidor X, em vez do XRender. Ele é opcional e foi registrado em dois desktops AMD com radeonsi: a RX 9060 XT no XLibre com a 0.3.0-beta.1, e a Radeon Vega no Xorg com as verificações de recursos desta beta, cantos arredondados incluídos. Onde ele não conseguir iniciar, ou quando um quadro falhar, o Compust registra um aviso e usa o XRender pelo resto da sessão. Relatos com drivers Intel e NVIDIA são especialmente úteis; inclua as linhas do log que informam o dispositivo ou o retorno ao XRender.

## Voltar ao compositor anterior

Encerre o Compust com SIGTERM, ou com Ctrl+C no terminal dele. Ele libera a tela e termina com sucesso. Depois inicie de novo o compositor anterior, por exemplo:

```sh
pkill -TERM -x compust
picom -b
```

Restaure qualquer linha de inicialização que você alterou. Encerrar a sessão também termina o Compust: quando o servidor X fecha, o Compust sai com `The X11 server closed the connection`, o que é esperado.

## Limitações conhecidas

- A suspensão da composição em tela cheia exige uma janela opaca e quadrada sobre a tela inteira, então nunca vale para um monitor entre vários.
- As sombras são pretas, e janelas com formato próprio não projetam sombra e mantêm os cantos quadrados. Bordas X arredondadas usam o pixel superior esquerdo da borda como cor em todos os cantos; gerenciadores que pintam cada lado com uma cor diferente não mantêm essas cores distintas nos cantos.
- Não há animações de movimento ou escala, opções globais para a opacidade das janelas ativas e inativas além das regras nem compatibilidade com a configuração do picom.
- As regras comparam texto idêntico. O foco vem do `_NET_ACTIVE_WINDOW`; com um gerenciador de janelas que não o define, como o Xmonad sem `XMonad.Hooks.EwmhDesktops`, toda janela conta como focada.
- Uma janela desfocada desfoca de novo todo o seu fundo sempre que algo abaixo dela muda. `blur_radius` é arredondado para 2, 4, 8 ou 16 pixels.
- O renderizador de GPU foi registrado em dois desktops. Neles, desenha os mesmos quadros que o XRender com diferença de até dois níveis de cor, usa cerca de 62 MiB a mais de memória e custa mais que o XRender enquanto janelas são redimensionadas.
- Um processo compõe uma tela X. Vários monitores compartilham uma superfície, e o Present segue o ritmo de um dos monitores.
- O Present copia um único buffer. A ocorrência de tearing depende do driver: o driver modesetting do XLibre ativa o TearFree por padrão, e o do Xorg 21.1.24 não tem TearFree.

## Relatar um problema

Colete a versão, o diagnóstico e a disposição dos monitores:

```sh
compust --version
compust --diagnose
xrandr --current
```

Anote a distribuição, o servidor X e a versão, o gerenciador de janelas e a versão, a GPU e o driver (por exemplo, pelo `lspci -k`) e o seu arquivo de configuração. Reproduza o problema com logs de depuração:

```sh
RUST_LOG=compust=debug compust 2>compust.log
```

Descreva os passos, o resultado esperado e o observado e se encerrar o Compust restaurou o desktop. Em seguida, abra um [relato de falha](https://github.com/hashdefault/compust/issues/new?template=bug.yml) com essas informações. Antes, remova dados privados dos logs e das capturas de tela.
