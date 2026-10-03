# Teste da beta

[English (US)](BETA.md) | [Português (Brasil)](BETA.pt-BR.md)

O Compust 0.2.0-beta.2 é a segunda beta, voltada a testes controlados da comunidade. Ele ainda não substitui o picom. O suporte é declarado somente onde há evidências registradas; outras configurações são bem-vindas para testes, mas continuam sem validação.

## Escopo declarado

Duas combinações registradas estão validadas: XLibre no desktop AMD e Xorg no laptop Intel.

| Área | Validado nesta beta | Evidências |
| --- | --- | --- |
| Servidor X | XLibre 25.1.9 ou Xorg 21.1.11 em sessão nativa | [Sessão de desktop AMD](DESKTOP_TESTING.pt-BR.md#sessão-de-desktop-registrada-em-hardware-2026-10-03) e [sessões Intel](DESKTOP_TESTING.pt-BR.md#sessões-registradas-em-intelxorg-2026-10-03) |
| Gerenciador de janelas | Xmonad 0.18.1 (AMD) ou 0.17.2 (Intel) com EWMH | Mesmas sessões |
| GPU e driver | AMD Radeon Vega (Ryzen 5 5600GT) com Mesa 26.2.4, ou Intel Iris Plus G1 (Core i3-1005G1) com Mesa 25.2.8; ambas com o driver modesetting e glamor | Mesmas sessões |
| Monitores | AMD: uma ou duas saídas em 1920×1080 a 60 Hz, incluindo desconexão e reconexão. Intel: um painel de laptop de 1366×768 a 60 Hz, incluindo uma mudança de modo | [Sessão de monitores](DESKTOP_TESTING.pt-BR.md#sessão-registrada-em-hardware-2026-10-03) e sessões Intel |
| Renderização | Present e XRender direto, fades, transparência e desfoque | [Sessão do desfoque em pirâmide](DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03) e sessões Intel |

Xorg 21.1 e XLibre 25.1 também passam nos cenários de desktop aninhados no Xephyr, o que cobre o comportamento do protocolo, mas não drivers nem monitores. GPUs NVIDIA, outros gerenciadores de janelas, Xorg com AMD, XLibre com Intel, hotplug físico em Intel, taxas de atualização mistas e HDR não foram testados. Relatos dessas configurações são especialmente úteis.

## Instalar

Baixe `compust-0.2.0-beta.2-x86_64-linux.tar.gz` e `SHA256SUMS` na [página da versão](https://github.com/hashdefault/compust/releases/tag/v0.2.0-beta.2), depois verifique e extraia os arquivos:

```sh
sha256sum -c SHA256SUMS
tar -xzf compust-0.2.0-beta.2-x86_64-linux.tar.gz
cd compust-0.2.0-beta.2-x86_64-linux
./compust --version
```

O binário exige Linux x86_64 com glibc 2.34 ou mais recente. `BUILDINFO` registra a revisão do código, o compilador e o comando de compilação. Para compilar a partir do código-fonte, instale a toolchain Rust fixada pelo projeto e execute:

```sh
git clone --branch v0.2.0-beta.2 https://github.com/hashdefault/compust.git
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
compust --check-config --config ~/.config/compust/compust.toml
```

O Compust só lê uma configuração passada em `--config`; o caminho acima é uma convenção. Pare o compositor atual e inicie o Compust em um terminal da mesma sessão X:

```sh
pkill -x picom
compust --config ~/.config/compust/compust.toml
```

Se outro compositor ainda controlar a tela, o Compust se recusa a iniciar e informa o motivo. Para iniciá-lo com o Xmonad, substitua a linha de inicialização do seu compositor, como `spawnOnce "picom ..."`, pela linha a seguir:

```haskell
spawnOnce "compust --config $HOME/.config/compust/compust.toml"
```

Com `~/.xinitrc`, inicie `compust --config ~/.config/compust/compust.toml &` antes do gerenciador de janelas.

## Voltar ao compositor anterior

Encerre o Compust com SIGTERM, ou com Ctrl+C no terminal dele. Ele libera a tela e termina com sucesso. Depois inicie de novo o compositor anterior, por exemplo:

```sh
pkill -TERM -x compust
picom -b
```

Restaure qualquer linha de inicialização que você alterou. Encerrar a sessão também termina o Compust: quando o servidor X fecha, o Compust sai com `The X11 server closed the connection`, o que é esperado.

## Limitações conhecidas

- Cada evento de dano redesenha a tela inteira. Um desktop ocioso continua ocioso, mas telas grandes ou movimentadas custam mais.
- Cada janela translúcida desfoca a própria área, então muitas janelas translúcidas sobrepostas multiplicam esse trabalho. `blur_radius` é arredondado para 2, 4, 8 ou 16 pixels.
- Janelas em tela cheia continuam sendo compostas; não há suspensão da composição para jogos ou vídeos.
- Não há sombras, cantos arredondados, animações de movimento ou escala, regras por janela, recarga de configuração nem compatibilidade com a configuração do picom.
- Um processo compõe uma tela X. Vários monitores compartilham uma superfície, e o Present segue o ritmo de um dos monitores.
- O Present copia um único buffer. A ocorrência de tearing depende do driver; o driver modesetting do XLibre ativou o TearFree por padrão na máquina registrada.

## Relatar um problema

Colete a versão, o diagnóstico e a disposição dos monitores:

```sh
compust --version
compust --diagnose
xrandr --current
```

Anote a distribuição, o servidor X e a versão, o gerenciador de janelas e a versão, a GPU e o driver (por exemplo, pelo `lspci -k`) e o seu arquivo de configuração. Reproduza o problema com logs de depuração:

```sh
RUST_LOG=compust=debug compust --config ~/.config/compust/compust.toml 2>compust.log
```

Descreva os passos, o resultado esperado e o observado e se encerrar o Compust restaurou o desktop. Em seguida, abra um [relato de falha](https://github.com/hashdefault/compust/issues/new?template=bug.yml) com essas informações. Antes, remova dados privados dos logs e das capturas de tela.
