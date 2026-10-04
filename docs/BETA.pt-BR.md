# Teste da beta

[English (US)](BETA.md) | [Português (Brasil)](BETA.pt-BR.md)

O Compust 0.3.0-beta.1 é a quarta beta, voltada a testes controlados da comunidade. Ele ainda não substitui o picom. O suporte é declarado somente onde há evidências registradas; outras configurações são bem-vindas para testes, mas continuam sem validação.

Esta beta pinta de forma diferente das betas 0.2.0: ela repinta apenas a área que mudou, guarda cada fundo desfocado até que algo abaixo dele mude e pula janelas escondidas atrás de janelas opacas. Ela também traz descoberta e recarga de configuração, regras por janela, desfoque ponderado pela opacidade de cada pixel e um renderizador de GPU opcional. As [notas da versão](releases/v0.3.0-beta.1.md) listam as mudanças.

## Escopo declarado

Uma máquina registrada rodou o código de renderização desta beta: um desktop AMD com Radeon RX 9060 XT, no XLibre com dwm.

| Área | Registrado com o código de renderização desta beta | Evidências |
| --- | --- | --- |
| Servidor X | XLibre 25.1.9 em sessão nativa | Sessões de [repintura por regiões](DESKTOP_TESTING.pt-BR.md#repintura-por-regiões-registrada-2026-10-04), [reaproveitamento do desfoque](DESKTOP_TESTING.pt-BR.md#reaproveitamento-do-desfoque-registrado-2026-10-04), [oclusão](DESKTOP_TESTING.pt-BR.md#oclusão-registrada-2026-10-04) e [renderizador de GPU](DESKTOP_TESTING.pt-BR.md#renderizador-de-gpu-registrado-2026-10-04) |
| Gerenciador de janelas | dwm 6.8 | Mesmas sessões |
| GPU e driver | AMD Radeon RX 9060 XT com radeonsi no Mesa 26.2.4 e o driver modesetting | Mesmas sessões |
| Monitores | Uma ou duas saídas em 1920×1080 a 60 Hz | Mesmas sessões |
| Renderização | Present com repintura por regiões, reaproveitamento do desfoque e oclusão; o renderizador de GPU opcional ao lado do XRender | Mesmas sessões |

Essas sessões rodam cenas de benchmark com janelas sintéticas, em commits anteriores à versão; nenhuma foi repetida com o binário desta versão. Os cenários de desktop do probe, a cópia direta via XRender, o hotplug físico e a suspensão e retomada não foram registrados nessa máquina, e as regras por janela e o desfoque ponderado têm apenas testes automatizados. Uma [sessão de recarga](DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-recarga-2026-10-03) na mesma máquina, registrada antes das mudanças de renderização, cobre recargas de configuração enquanto uma saída é ligada e desligada e o layout muda.

As duas máquinas validadas na 0.2.0-beta.3 não rodaram esta beta: o desktop AMD com Radeon Vega e Xmonad, e o laptop Intel com Iris Plus e Xmonad, Openbox e i3. Seus registros, listados na [matriz de compatibilidade](ROADMAP.pt-BR.md#matriz-de-compatibilidade), cobrem gerenciamento de janelas, hotplug físico e suspensão e retomada com o pintor anterior. GPUs NVIDIA, outros gerenciadores de janelas, Xorg com AMD, XLibre com Intel, taxas de atualização claramente diferentes, mais de dois monitores e HDR não foram testados. Relatos de qualquer uma dessas configurações são especialmente úteis.

## Instalar

Baixe `compust-0.3.0-beta.1-x86_64-linux.tar.gz` e `SHA256SUMS` na [página da versão](https://github.com/hashdefault/compust/releases/tag/v0.3.0-beta.1), depois verifique e extraia os arquivos:

```sh
sha256sum -c SHA256SUMS
tar -xzf compust-0.3.0-beta.1-x86_64-linux.tar.gz
cd compust-0.3.0-beta.1-x86_64-linux
./compust --version
```

O binário exige Linux x86_64 com glibc 2.34 ou mais recente. `BUILDINFO` registra a revisão do código, o compilador e o comando de compilação. Para compilar a partir do código-fonte, instale a toolchain Rust fixada pelo projeto e execute:

```sh
git clone --branch v0.3.0-beta.1 https://github.com/hashdefault/compust.git
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

Com `~/.xinitrc`, inicie `compust &` antes do gerenciador de janelas.

Depois de editar a configuração, aplique-a sem reiniciar:

```sh
pkill -USR1 -x compust
```

Um arquivo que não puder ser lido ou for inválido é rejeitado com um aviso, e as opções em uso continuam valendo. O [README](../README.pt-BR.md#configuração) descreve todas as opções e as [regras por janela](../README.pt-BR.md#regras-por-janela).

### Experimentar o renderizador de GPU

`backend = "gl"` na configuração desenha com OpenGL ES na GPU do servidor X, em vez do XRender. Ele é opcional e foi registrado em um desktop, com radeonsi. Onde ele não conseguir iniciar, ou quando um quadro falhar, o Compust registra um aviso e usa o XRender pelo resto da sessão. Relatos com drivers Intel e NVIDIA são especialmente úteis; inclua as linhas do log que informam o dispositivo ou o retorno ao XRender.

## Voltar ao compositor anterior

Encerre o Compust com SIGTERM, ou com Ctrl+C no terminal dele. Ele libera a tela e termina com sucesso. Depois inicie de novo o compositor anterior, por exemplo:

```sh
pkill -TERM -x compust
picom -b
```

Restaure qualquer linha de inicialização que você alterou. Encerrar a sessão também termina o Compust: quando o servidor X fecha, o Compust sai com `The X11 server closed the connection`, o que é esperado.

## Limitações conhecidas

- Janelas em tela cheia continuam sendo compostas; não há suspensão da composição para jogos ou vídeos.
- Não há sombras, cantos arredondados, animações de movimento ou escala nem compatibilidade com a configuração do picom.
- As regras comparam texto idêntico e não escolhem janelas pelo foco.
- Uma janela desfocada desfoca de novo todo o seu fundo sempre que algo abaixo dela muda. `blur_radius` é arredondado para 2, 4, 8 ou 16 pixels.
- O renderizador de GPU foi registrado em um desktop. Nele, desenha os mesmos quadros que o XRender com diferença de até dois níveis de cor, usa cerca de 62 MiB a mais de memória e custa mais que o XRender enquanto janelas são redimensionadas.
- Um processo compõe uma tela X. Vários monitores compartilham uma superfície, e o Present segue o ritmo de um dos monitores.
- O Present copia um único buffer. A ocorrência de tearing depende do driver; o driver modesetting do XLibre ativou o TearFree por padrão na máquina registrada com Radeon Vega.

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
