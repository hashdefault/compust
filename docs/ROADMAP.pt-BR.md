# Roteiro de desenvolvimento

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

O objetivo é tornar este compositor mínimo em Rust uma opção prática para usuários de Xorg e XLibre; o [marco 1.0](#10-versão-estável) define o que isso exige. O progresso é medido pelos próprios registros de correção, confiabilidade, latência e uso de recursos do Compust. Reescrever um recurso em Rust, por si só, não comprova melhor desempenho.

## Base 0.1: implementada

O repositório contém um compositor executável com composição XRender, transições de opacidade, transparência alfa, desfoque, recorte por formato, acompanhamento de atualizações, empilhamento, tratamento de redimensionamento de janelas, propriedades de papel de parede e apresentação opcional por cópia com Present. A suíte verifica o comportamento de um servidor real no Xvfb. A documentação e as orientações de contribuição estão em inglês e pt-BR.

Esse marco cria uma base para experimentação. Ele não comprova compatibilidade completa com ambientes gráficos nem desempenho em hardware real.

## Primeira beta: quatro etapas

A primeira beta usa o backend XRender e declara suporte somente aos ambientes com evidências registradas de teste. Ela é voltada a testes controlados da comunidade. A versão 0.2.0-beta.1 foi essa beta, a 0.2.0-beta.2 e a 0.2.0-beta.3 vêm em seguida, e a [0.3.0-beta.1](#quarta-pré-versão-beta) inicia o caminho até a 1.0; o [guia da beta](BETA.pt-BR.md) lista o escopo declarado atual.

| Etapa | Estado | Resultado necessário |
| --- | --- | --- |
| 1. Estabilidade das janelas | Concluída no Xvfb | Ciclo de vida, menus, transições de tela cheia e propriedades inválidas com cobertura reproduzível, sem quedas nem janelas invisíveis ou imagens antigas. |
| 2. Monitores e recursos | Verificada no Xvfb, em um desktop AMD/XLibre e em um laptop Intel/Xorg, cada um com hotplug físico; demais hardwares pendentes | Mudanças de resolução, conexão/desconexão de monitores, recuperação da apresentação e consumo de recursos em redimensionamentos repetidos verificados. |
| 3. Desktops reais | Cenários do Xmonad registrados em servidores aninhados, em um desktop AMD/XLibre e em um laptop Intel/Xorg; cenários do Openbox e do i3 nesse laptop; outros gerenciadores e drivers pendentes | Sessões Xorg/XLibre com registro de gerenciadores e drivers testados, além de medições de CPU, memória e regularidade dos quadros. |
| 4. Distribuição da beta | Publicada como v0.2.0-beta.1, v0.2.0-beta.2 e v0.2.0-beta.3, cada uma para seu escopo declarado | Pré-lançamento versionado com instruções de instalação e execução, limitações conhecidas, artefatos verificados e procedimento reproduzível para relatar falhas. |

### 1. Estabilidade das janelas

Exercitar sequências rápidas de map/unmap/destroy, fades interrompidos, janelas decoradas, menus override-redirect, entrada e saída de tela cheia, propriedades malformadas e destruição durante requisições do protocolo. Começar por regressões determinísticas de pixels no Xvfb; registrar o comportamento de gerenciadores reais na etapa 3.

**Aceitação:** cada defeito reproduzido possui uma regressão que falha sem a correção; os cenários cobertos preservam os pixels corretos e mantêm o compositor em execução; suíte completa, formatação, Clippy e build de release passam. A cobertura abaixo inclui clientes e molduras, propriedades, sequências rápidas, fades interrompidos, formatos extremos ou fora da tela e destruição antes e entre requisições de captura. A validação automatizada no Xvfb está concluída; a validação de gerenciadores e drivers reais continua na etapa 3.

### 2. Monitores e recursos

Testar mudanças de resolução via RandR e hotplug físico, recuperação das falhas de apresentação previstas e redimensionamentos repetidos com contagem de recursos do servidor X. Registrar a configuração e a disposição dos monitores utilizadas.

**Aceitação:** a imagem se recupera após cada transição coberta, a apresentação continua e as operações repetidas não provocam crescimento indefinido de memória ou recursos do servidor.

Os cenários com Xvfb descritos abaixo passam. Uma sessão XLibre nativa em hardware AMD também passou na desconexão e reconexão física dos dois conectores e em mudanças de modo e disposição com dois monitores, nos dois modos de apresentação. [HDMI-1 a 60 Hz e DP-2 a 180 Hz](DESKTOP_TESTING.pt-BR.md#monitores-com-taxas-diferentes-registrados-2026-10-04) agora têm registro na RX 9060 XT em XRender e GL. Outros drivers e servidores e mais de dois monitores precisam de seus próprios registros; desativar um CRTC virtual não comprova o comportamento físico.

### 3. Desktops reais

Executar cenários documentados com gerenciadores de janelas reais em Xorg e XLibre. Registrar servidor, gerenciador, GPU/driver, configuração e commit exato. Medir CPU ociosa e em atividade, memória e regularidade dos quadros; corrigir falhas nos ambientes propostos para suporte na beta.

**Aceitação:** publicar uma matriz de compatibilidade com evidências para cada ambiente anunciado, uma referência reproduzível de medições e as limitações restantes. Combinações de servidor e driver ainda não testadas permanecem sem validação.

O [guia de validação de desktops](DESKTOP_TESTING.pt-BR.md) documenta o script isolado, as medições e o procedimento em hardware. As execuções registradas de Xephyr/Xmonad iniciam esta etapa; elas não concluem os requisitos de drivers e monitores físicos. Uma sessão AMD/XLibre dedicada agora executa em hardware os cenários do probe, a troca de papel de parede, os fades e o desfoque com translucidez. Aplicativos reais, gerenciadores com decoração ou reparenting, Xorg em hardware e outros drivers continuam em aberto.

### 4. Distribuição da beta

Publicar uma prévia beta versionada com instruções de compilação ou instalação do binário, exemplos de configuração, orientações de início e encerramento, checksums dos artefatos distribuídos, limitações conhecidas e um modelo de relato com diagnóstico e passos de reprodução.

**Aceitação:** uma pessoa consegue instalar e executar a versão exata, retornar ao compositor anterior e relatar uma falha seguindo as instruções fornecidas. O CI passa para o commit da versão, e as três etapas anteriores estão aprovadas dentro do escopo de suporte declarado.

A versão 0.2.0-beta.1 concluiu esses quatro critérios de liberação para seu escopo declarado. A expansão de backend de GPU e os efeitos avançados podem vir depois da primeira beta. O [marco de renderização](#trabalho-de-renderização-quatro-etapas-concluídas-em-um-desktop) veio em seguida, e a [1.0](#10-versão-estável) é o marco atual; a ampliação da cobertura de hardware da beta continua em paralelo, conforme chegam relatos.

## Progresso e verificação

### Etapa 1 da 1.0: recursos registrados em AMD/XLibre

`desktop_probe --features` agora verifica uma cena fixa sem gerenciador de janelas: cada pixel da cena de sombras contra um perfil independente, mudanças de foco controladas pelo `_NET_ACTIVE_WINDOW` e suspensão e retomada em tela cheia com a mesma carga de desenho e registros separados de CPU. [`tools/features-check.sh`](../tools/features-check.sh) executa XRender e GL, compara suas imagens de sombras e foco e rejeita uma execução GL que volte ao XRender. [`tools/hardware-session.sh --features`](DESKTOP_TESTING.pt-BR.md#verificar-sombras-foco-e-suspensão-em-tela-cheia) inicia esse procedimento em uma sessão dedicada de hardware. O probe passou no Xvfb, inclusive com configurações erradas de propósito, e depois passou nos dois pintores na [sessão em hardware RX 9060 XT / XLibre](DESKTOP_TESTING.pt-BR.md#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04). Cada pixel da cena de sombras ficou a no máximo um nível RGB da referência independente, as imagens de foco foram idênticas entre pintores e a suspensão interrompeu Present e Damage do overlay durante 600 atualizações por fase. O Compust não acumulou ticks de CPU durante os dez segundos suspenso, contra 0,3% de um núcleo durante a composição no XRender e 0,6% no GL. Retomada por popup, recaptura atualizada e encerramento passaram. Esse registro usa uma tela e janelas sintéticas sem gerenciador de janelas; aplicativos reais e outros equipamentos ainda precisam de validação.

### Etapa 1 da 1.0: regras pelo foco

As regras ganham um seletor `focused`: `true` escolhe a janela que o gerenciador de janelas informa como ativa no `_NET_ACTIVE_WINDOW` da raiz, e `false` todas as outras, para que uma regra possa esmaecer janelas inativas como faz o `inactive-opacity` do picom. A propriedade indica o cliente, então uma moldura está focada quando seu cliente está. Uma mudança nela custa uma leitura e resolve as regras de novo para as janelas que perderam e ganharam o foco. Em um gerenciador de janelas que não define a propriedade, toda janela conta como focada, de modo que uma regra para janelas sem foco não muda nada, em vez de esmaecer o desktop inteiro. No Xvfb do Xorg, com um terminal aberto, Openbox 3.6.1, i3 4.25.1, bspwm 0.9.12 e dwm 6.8 definiram a propriedade, e o Xmonad 0.18.1 a definiu com `XMonad.Hooks.EwmhDesktops` e não com sua configuração padrão.

Seis testes X11 em [focus.rs](../tests/cases/focus.rs) cobrem a regra acompanhando a janela ativa, sem janela ativa, sem a propriedade e com uma de tipo errado; uma janela mapeada enquanto outra está ativa; janelas abertas antes de o Compust iniciar; o foco mantido após mudança de título, redimensionamento, recarga e suspensão da composição; o foco combinado com um seletor de tipo; um cliente dentro de uma moldura, inclusive um encontrado depois que a moldura apareceu; e uma leitura de propriedade por mudança de foco. Cada uma de 10 mudanças na implementação, feitas uma de cada vez e desfeitas, fez pelo menos um deles falhar. Com isso, os três recursos da etapa 1 estão implementados; suas cenas fixas agora têm um [registro em hardware nos dois pintores](DESKTOP_TESTING.pt-BR.md#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04), com o foco controlado pelo probe. Com o Xvfb do Xorg, passam os 120 testes X11, os 35 unitários e os 6 de CLI, além dos 9 testes do crate de GPU, com formatação e Clippy estrito.

### Etapa 1 da 1.0: suspensão da composição em tela cheia

Com `unredirect_fullscreen = true`, a composição é suspensa enquanto a janela do topo entre as exibidas é opaca, não tem formato e cobre a tela inteira: o Compust deixa de redirecionar as janelas, que passam a desenhar direto na tela, e desmapeia seu overlay. Ela volta quando uma janela visível é mapeada, quando a janela que cobre a tela se move, muda de tamanho, de formato ou de opacidade, é reempilhada abaixo de outra ou fecha, e quando a opção é recarregada como desligada. O plano permitia janelas opacas acima dela. Agora qualquer coisa acima retoma a composição, para que um menu ou uma notificação mantenha sombra e fade, e a regra continua sendo que uma janela esconde todo o resto. Uma janela que não recebe desenho não muda nada ao ser mapeada acima dela. A opção vem desligada.

A retomada redireciona cada janela para um pixmap novo que começa com o que a janela mostra; por isso cada superfície mapeada é capturada de novo, e uma janela que fechou enquanto a composição estava suspensa sai sem fade, porque o Compust só tem a imagem dela de antes. Enquanto a composição está suspensa, o Compust mantém seus buffers e os pixmaps antigos das superfícies; liberá-los fica para depois.

Oito testes X11 em [unredirect.rs](../tests/cases/unredirect.rs) cobrem uma janela que cobre a tela ao ser mapeada e desmapeada, com Present e com XRender; o que uma janela desenhou durante a suspensão, exibido assim que outra janela é mapeada acima dela; cada mudança que descobre a tela, seguida dos pixels corretos; janelas com canal alfa, com um pixel a menos que a tela e só de entrada; uma janela que cobre a tela ao fim de um fade; uma destruída durante a suspensão; e as contagens de recursos ao longo de cinco ciclos. Cada uma de 14 mudanças na implementação, feitas uma de cada vez e desfeitas, fez pelo menos um deles falhar. Essas regressões rodaram no Xvfb. O [probe em hardware](DESKTOP_TESTING.pt-BR.md#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04) depois registrou suspensão, uso de CPU e recaptura atualizada nos dois pintores em uma tela AMD/XLibre; ainda falta registrar como aplicativos reais em tela cheia entram e saem dela. Com o Xvfb do Xorg, passam os 114 testes X11, os 35 unitários e os 6 de CLI, além dos 9 testes do crate de GPU, com formatação e Clippy estrito.

### Etapa 1 da 1.0: sombras

As janelas projetam sombra quando `shadow_radius` é maior que zero: uma cópia preta do retângulo da janela, deslocada por `shadow_offset_x` e `shadow_offset_y`, desfocada ao longo do raio e tão escura quanto `shadow_opacity` vezes a opacidade da janela. As sombras vêm desligadas, então uma atualização não muda nenhum desktop. Uma janela projeta sombra quando seu tipo é normal, dialog, utility, splash ou toolbar. A janela que não declara tipo e que o gerenciador de janelas não controla não projeta, nem aquela cujo `_GTK_FRAME_EXTENTS` declara margens para uma sombra própria, e `shadow` em uma regra decide nos dois sentidos. Janelas com formato nunca projetam sombra: uma sombra que acompanhe o formato espera pelos cantos arredondados, depois da 1.0.

Desfocar um retângulo se separa em um perfil ao longo de cada eixo; por isso cada pintor guarda duas tiras por janela e as multiplica em cada pixel, o que custa ao XRender duas composições pequenas e ao pintor de GPU um desenho. As tiras do XRender têm folga de tamanho, então um redimensionamento envia perfis novos sem alocar. Três filtros de caixa calculados com inteiros dão os perfis, iguais nos dois pintores. A sombra fica em volta da janela e nunca embaixo dela, é pintada depois que o desfoque da própria janela leu a cena e faz parte do que um quadro mostrou da janela: movimentos, fades e recargas repintam as duas, enquanto o conteúdo da própria janela não mexe na sombra. Uma janela totalmente escondida atrás de uma opaca ainda mostra a parte da sombra que passa dela.

Doze testes X11 em [shadows.rs](../tests/cases/shadows.rs) conferem a extensão, o deslocamento e o perfil da sombra pixel a pixel, inclusive onde ela começa fora da tela; que uma janela translúcida continua tão clara quanto sem sombras; quais tipos, margens, formatos e regras projetam sombra, e que cada um deles muda a sombra de uma janela aberta; fades, recargas e redimensionamentos sem vazamento de buffers, e sem buffer novo em um passo pequeno; sombras em volta de uma janela escondida e de um desfoque escondido; e, depois de cada tipo de mudança em uma cena com uma janela desfocada, um quadro igual ao de um renderizador novo, com Present e com XRender. Cada uma de 21 mudanças na implementação, feitas uma de cada vez e desfeitas, fez pelo menos um deles falhar. O teste do crate de GPU desenha uma sombra a partir de dois perfis no dispositivo de software do Mesa e, onde existe um nó de renderização, nessa GPU, onde passou com radeonsi. O [probe em hardware](DESKTOP_TESTING.pt-BR.md#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04) agora confere o pintor de GPU e o XRender com glamor contra uma referência independente de sombras e compara suas imagens da tela inteira. Essas cenas passaram na RX 9060 XT com diferença de até um nível RGB; o desempenho das sombras e outros equipamentos ainda precisam de registro. Com o Xvfb do Xorg, passam os 106 testes X11, os 35 unitários e os 6 de CLI, além dos 9 testes do crate de GPU, com formatação e Clippy estrito.

### Uso cotidiano: descoberta e recarga de configuração

O uso da beta mostrou que mudar `fade_ms` exigia reiniciar e que uma configuração só era lida quando `--config` a indicava. Sem `--config`, o Compust agora lê o primeiro `compust/compust.toml` em `$XDG_CONFIG_HOME` (padrão `~/.config`) e depois em `$XDG_CONFIG_DIRS` (padrão `/etc/xdg`), e `--check-config` informa o arquivo encontrado. SIGUSR1 recarrega a configuração que uma reinicialização leria. Um arquivo ilegível ou inválido é registrado no log, e a configuração em uso permanece. Um `fade_ms` recarregado vale para toda transição iniciada depois, e uma mudança de desfoque ou vsync substitui o renderizador assim que o Present libera seu buffer. Um envio rejeitado pelo Present agora marca a extensão como indisponível na sessão, em vez de desligar `vsync`; assim, uma recarga não traz de volta um caminho que o servidor recusou.

Sete testes X11 em [reload.rs](../tests/cases/reload.rs) cobrem uma recarga de opacidade; uma duração de fade recarregada fechando uma janela aberta antes dela; recargas rejeitadas com valor fora do intervalo, erro de sintaxe e arquivo removido; desfoque alternado dezesseis vezes em cada modo de apresentação sem mudança nas contagens do XRes nem nos bytes de pixmaps; troca entre Present e cópia direta; e uma recarga depois de uma rejeição do Present. Cada um falhou quando sua parte do comportamento foi desativada em uma cópia temporária. Testes unitários e de CLI cobrem a ordem de busca, um link quebrado e a precedência de `--config` sobre a descoberta.

A suíte rodou com o Xvfb 25.1.9 do XLibre, porque o do Xorg conflita com os pacotes do XLibre naquela máquina. Os sete testes existentes que reconfiguram o CRTC via RandR falham ali antes e depois desta mudança: esse Xvfb informa um CRTC de 1280×1024 em uma tela de 320×240 e rejeita `SetCrtcConfig` com `BadValue`. Os outros 58 testes X11, 12 testes unitários e 6 testes de CLI passam, com formatação e Clippy estrito; a CI usa o Xvfb do Xorg. Uma [sessão ao vivo](DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-recarga-2026-10-03) em uma terceira máquina, XLibre com uma Radeon RX 9060 XT sob dwm, recarregou desfoque, fades e vsync e rejeitou um arquivo inválido com um e com dois monitores. Todo intervalo do Present durou um vblank, e estados repetidos mantiveram bytes idênticos de pixmaps próprios.

### Uso cotidiano: recarga ao salvar

Aplicar uma configuração editada ainda exigia um sinal. Agora o Compust também recarrega quando o arquivo é salvo: [watch.rs](../src/watch.rs) observa, pelo inotify, o diretório de cada arquivo que a busca leria e o do arquivo para o qual um link simbólico entre eles aponta, e recarrega depois que os arquivos ficam 100 ms sem mudar, então um salvamento em várias etapas recarrega uma vez. O SIGUSR1 continua recarregando, e segue sendo o único meio onde o inotify não está disponível e para um diretório de configuração criado depois que o Compust inicia.

Cinco testes unitários cobrem salvamentos no lugar, por renomeação e por remoção; outros arquivos no mesmo diretório; a espera de um salvamento que primeiro move o arquivo antigo; links para outro diretório; e a troca dos arquivos observados. Um teste X11 em [reload.rs](../tests/cases/reload.rs) salva a configuração no lugar, renomeando um arquivo novo sobre ela e como o Vim faz, sem sinal, e espera cada opacidade aparecer; cópias temporárias que nunca deixam uma mudança assentar ou que perdem renomeações fazem o teste falhar. Num Xvfb privado, o binário de release registrou exatamente uma recarga, cerca de 100 ms depois da última escrita e sem aviso, para cada um desses salvamentos e para uma rajada de três escritas em 60 ms. Na sessão i3 4.25.1 de costume do desktop com Radeon Vega, no Xorg 21.1.24, o build de release instalado de `ed2cdb0` (`cc8ad9e1…`) registrou uma recarga em até 0,6 segundo depois que o arquivo de configuração foi regravado sem mudanças, e o responsável pelo projeto relata que as edições salvas se aplicam no uso diário; essa verificação não foi arquivada. Os 123 testes X11 passam com o Xvfb 21.1.24 do Xorg, junto com 45 testes unitários e 6 de CLI, Clippy estrito, o build de release e as verificações de recursos.

### Uso cotidiano: regras por janela e desfoque ponderado

Um halo fosco em volta dos menus de clique direito do Brave no desktop AMD/XLibre, também sob o picom, vinha de desfocar atrás de janelas de 32 bits inteiras: a janela do menu se estende 24 pixels além do menu nas laterais, 12 em cima e 36 embaixo, com uma sombra de no máximo 16% de opacidade. O fundo desfocado agora aparece em cada pixel com a força com que a janela o cobre, seu alfa vezes sua opacidade, nos dois pintores; assim, margens transparentes ficam nítidas sem configuração, e o desfoque surge e some com a janela. Um teste em [backdrops.rs](../tests/cases/backdrops.rs) mapeia uma janela parecida com um menu cuja margem precisa deixar o fundo nítido.

Regras por janela escolhem janelas por classe de recurso, tipo EWMH ou título, com texto exato, e definem sua opacidade, seu desfoque e a duração do fade; cada opção vem da primeira regra compatível que a define. Uma captura lê a identidade do cliente em uma única ida e volta, e um redimensionamento a mantém sem lê-la de novo. Mudanças nessas propriedades, um cliente encontrado depois e uma recarga resolvem as regras de novo. Uma moldura que perde seu cliente, e uma janela que já sumiu quando sua última mudança é lida, mantêm a identidade; assim, uma janela em fechamento termina o fade com as próprias regras. Uma janela sem tipo recebe o padrão da EWMH, `dialog` quando é transitória e controlada pelo gerenciador de janelas e `normal` nos demais casos, enquanto o [plano de animações](#configuração-e-elegibilidade-propostas) propunha que um tipo ausente não correspondesse a nada.

Oito testes X11 em [rules.rs](../tests/cases/rules.rs) cobrem uma regra de classe substituindo a opacidade global por meio de uma moldura, antes e depois de `WM_STATE` e depois que o cliente é destruído; uma janela destruída logo após mudar de título; `_NET_WM_NAME` antes de `WM_NAME` como `STRING` Latin-1 ou UTF-8; tipos conhecidos, desconhecidos e ausentes, com janelas transitórias e override-redirect; desfoque desligado e mantido por regras, e ambos invertidos por uma recarga; um fade lento na abertura, no fechamento e na reabertura; regras de opacidade acrescentadas e removidas por recargas; e quatro redimensionamentos que não leem a classe, enquanto uma mudança de título a lê uma vez. Cada uma de 19 alterações na implementação, feitas em cópias temporárias, fez falhar ao menos um deles. Escrever o teste da janela destruída revelou um defeito: um erro na primeira das leituras de propriedades em lote retornava cedo e deixava os erros das outras respostas chegarem como eventos, que param o compositor; agora todas as respostas são lidas antes. Testes unitários cobrem validação, comparação, precedência e a leitura de `WM_CLASS`. Com o Xvfb do Xorg, passam todos os 94 testes X11, 33 testes unitários e 6 testes de CLI, assim como os 7 testes do crate de GPU no dispositivo de software do Mesa, com formatação e Clippy estrito.

### Teste local da beta: bordas completas no Xmonad

O uso diário após `v0.2.0-beta.1` revelou bordas direitas e inferiores ausentes. Em uma janela sem formato delimitador definido pelo cliente, `ShapeGetRectangles` retornava dimensões menores que o pixmap capturado por uma largura de borda. O Compust agora usa os limites completos do pixmap nessas janelas e preserva os formatos explícitos do cliente.

A [regressão de bordas fora da tela](../tests/cases/shapes.rs) existente falhou antes da correção e passa depois. As [regressões de bordas](../tests/cases/borders.rs) cobrem mudanças de cor por foco, redimensionamento com mudança da largura da borda e remoção de formato personalizado. Todos os 68 testes, formatação, Clippy estrito e build de release passam. Na sessão local do Xmonad, as duas janelas do Alacritty mantêm as quatro faixas de borda de 2 pixels com e sem foco. Isso continua os testes da beta; as tarefas de animação abaixo permanecem planejadas.

### Teste local da beta: janela parada após um redimensionamento restaurado

No laptop Intel/Xorg com i3, conectar ou desconectar um cabo HDMI congelava todas as janelas em tiling até que outra janela fosse aberta. O i3 reorganizava as janelas duas vezes em rápida sucessão e terminava nos tamanhos originais. O servidor X aloca um novo pixmap da janela a cada redimensionamento, mas o Compust comparava apenas o tamanho atual da janela com o capturado, não via mudança e continuava compondo o pixmap antigo, que não recebia mais desenho nem Damage. Uma gravação mostrou uma janela de teste em tiling parada por 26 segundos, da desconexão até que uma nova janela a redimensionasse, enquanto uma janela override-redirect continuava atualizando.

O Compust agora também compara o tamanho informado em cada `ConfigureNotify` e recaptura quando ele difere. Uma [regressão](../tests/cases/stability.rs) redimensiona e restaura uma janela sob um grab do servidor e depois a repinta; ela falhou antes da correção e passa depois. Com a correção, o mesmo teste com o cabo manteve a janela em tiling atualizando durante a desconexão e a reconexão. Todos os 70 testes, formatação, Clippy estrito e build de release passam.

O defeito não é específico de hotplug nem do i3: qualquer gerenciador de janelas que redimensione uma janela e a restaure antes de o Compust tratar o primeiro evento pode acioná-lo. Ele está presente na 0.2.0-beta.1 e na 0.2.0-beta.2. O executor de transições de monitores não o detectou, porque verificava apenas seu próprio marcador override-redirect. Agora ele também repinta uma janela gerenciada no lugar em cada amostra; com essa verificação, o binário da 0.2.0-beta.2 falha depois que uma saída é desligada sob o i3, e o compositor corrigido passa.

### Teste local da beta: tempo limite do Present

O Compust só reutiliza seu buffer do Present após os eventos de conclusão e liberação do envio anterior. Fora de uma mudança RandR, um evento perdido deixava a tela congelada enquanto o compositor continuava em execução. Nenhuma perda desse tipo foi observada em hardware; o risco foi encontrado em uma revisão do código. Um envio que não informa nada em um segundo agora é abandonado: o Compust substitui seus buffers e repinta o quadro uma vez. Se o envio seguinte também exceder o tempo limite, ele espera por novo dano antes de pintar de novo.

Uma nova regressão em [presentation.rs](../tests/cases/presentation.rs) retém os eventos de um envio sem nenhuma mudança de monitor. Ela falhou antes da correção, com a tela parada no quadro anterior, e passa depois. Todos os 69 testes, formatação, Clippy estrito e build de release passam. A troca de terminal virtual e a suspensão/retomada, em que essa perda é mais provável, continuam sem teste em hardware.

### Ciclo de vida de clientes e molduras: implementado

A associação do cliente agora acompanha criação tardia e remoção de `WM_STATE`, novos descendentes dentro de uma moldura existente, mudanças de parentesco entre molduras visíveis e a raiz, além da destruição do cliente. A opacidade da moldura mantém precedência. Um evento de opacidade pendente para um cliente já destruído não faz mais a moldura sobrevivente desaparecer.

Cinco testes de regressão de pixels em [client_lifecycle.rs](../tests/cases/client_lifecycle.rs) exercitam essas transições com o binário real do compositor. Os quatro cenários de ciclo de vida falharam antes da correção; desativar o novo tratamento de propriedades e a recuperação de `BadWindow` restrita ao cliente também reproduziu a opacidade desatualizada e o desaparecimento da moldura.

### Etapa 1 iniciada: validação de propriedades e sequências rápidas

`WM_STATE` agora exige o tipo declarado, formato de 32 bits e exatamente dois valores. A opacidade da janela exige um único `CARDINAL` de 32 bits. Propriedades vazias, truncadas, com valores excedentes ou tipo/formato incorreto são ignoradas; a opacidade inválida da moldura dá lugar à opacidade válida do cliente. Cinco regressões em [properties.rs](../tests/cases/properties.rs) falharam antes da correção e agora passam, incluindo a recuperação após substituir o estado malformado do cliente por dados válidos.

Quatro cenários adicionais em [stability.rs](../tests/cases/stability.rs) passam: 32 ciclos de map/unmap enfileirados seguidos de remapeamento em outra posição, destruição com eventos de opacidade/configuração/formato pendentes, remoção de um popup override-redirect e expansão/restauração de geometria do tamanho da tela. Eles verificam os pixels renderizados e que o compositor continua em execução. A cobertura de tela cheia aqui altera diretamente a geometria da janela em uma tela fixa do Xvfb; o comportamento de gerenciadores reais pertence à etapa 3, e a reconfiguração de monitores à etapa 2.

### Continuação da etapa 1: fades, formatos e destruição

Três defeitos foram reproduzidos e corrigidos. Remapear durante o fechamento agora captura o conteúdo novo e retoma a abertura a partir da opacidade corrente. Uma janela destruída mantém sua posição abaixo da antiga vizinha superior durante o fade, mesmo após outro evento de empilhamento. O formato permanece em coordenadas locais; a origem de recorte do XRender aplica a posição da janela sem saturar antecipadamente coordenadas negativas e deixar pixels pretos.

As regressões de retomada e empilhamento em [fades.rs](../tests/cases/fades.rs) e de coordenadas extremas em [shapes.rs](../tests/cases/shapes.rs) falharam antes das correções e passaram depois. Também estão cobertos: destruição durante a abertura, formatos vazios e separados, bordas de 20 pixels, recorte do desfoque fora da tela e uma janela de 4096×2048 movida completamente para fora da tela e de volta. Um teste unitário verifica continuidade, monotonicidade e ponto final exato ao reabrir o fade.

Três cenários em [destruction.rs](../tests/cases/destruction.rs) verificam 32 criações/map/destruições enfileiradas antes da captura, além de destruição com o evento de formato ou de geometria à frente da notificação de destruição. O bloqueio do servidor impõe a ordem; esses testes exercitam os tratamentos existentes sem ampliar os erros X11 ignorados. Eles não forçam a destruição em cada intervalo entre as requisições internas da captura.

Este conjunto levou a suíte a 44 testes aprovados. Os intervalos restantes entre requisições de captura são cobertos pelos casos abaixo.

### Etapa 1 concluída: destruição entre requisições de captura

Três testes em [capture_races.rs](../tests/cases/capture_races.rs) exercitam onze pontos: atributos da janela, geometria, nomeação do pixmap, criação da imagem, assinatura de eventos do cliente, `WM_STATE`, descoberta da árvore de clientes, assinatura de eventos de formato, criação de damage, retângulos do formato e opacidade. Um proxy local exclusivo dos testes pausa a requisição escolhida, conclui a destruição com confirmação por outra conexão ao Xvfb e encaminha os bytes originais. Respostas e eventos vêm do servidor real; não há instrumentação no código de produção nem esperas arbitrárias.

Cada caso espera a renderização de um marcador sobrevivente, verifica os pixels da janela inferior e que o compositor continua vivo, além de consultar os identificadores de pixmap, imagem e damage para comprovar a liberação. A recuperação existente passou sem mudanças na produção nem ampliação dos erros ignorados. Em cópias temporárias, desativar a liberação da imagem fez o teste falhar por recurso ainda alocado; desativar a recuperação da captura encerrou o compositor e fez o teste do sobrevivente falhar.

Isso concluiu a etapa 1 com 47 testes aprovados: seis unitários, três de CLI e trinta e oito de integração X11. Formatação, Clippy estrito e build de release passaram. Desktops reais e hotplug físico ainda exigem evidências próprias.

### Etapa 2: transições de monitores virtuais, recuperação da apresentação e recursos

Quatro testes em [monitors.rs](../tests/cases/monitors.rs) exercitam requisições RandR reais com Present e com cópia direta via XRender. A única saída do Xvfb é desativada, a tela raiz diminui de 320×240 para 240×180 e depois o tamanho original e a configuração do CRTC são restaurados. Dezesseis ciclos medidos preservam os pixels da janela sobrevivente, as contagens exatas de recursos XRes e os bytes totais reportados dos pixmaps do compositor. Casos separados desativam e restauram a saída sem alterar a resolução. Desativar a recriação do renderizador em uma cópia temporária faz ambos os casos de redimensionamento falharem.

Quatro testes de ciclo de vida em [resources.rs](../tests/cases/resources.rs) repetem redimensionamentos de janela mapeada e sequências de map/destroy 32 vezes por modo de apresentação, após aquecimento. As cenas restauradas mantêm exatamente as mesmas contagens de recursos e os bytes totais reportados dos pixmaps: 654.401 nos cenários de janelas e 878.401 na janela maior dos testes de monitores. Desativar a liberação de imagens em uma cópia temporária faz os quatro casos de ciclo de vida falharem no primeiro ciclo medido.

Um quinto teste de recursos mantém referências adicionais aos pixmaps a partir de outro cliente. O total de alocação permanece igual, enquanto a atribuição anterior, dividida por referências, cai de 641.066 para 429.600 bytes. O teste usa `QueryResourceBytes` do XRes 1.2 e exige tamanhos para todos os pixmaps; `QueryClientPixmapBytes` não serve para comparações exatas de alocação enquanto as referências de Present variam. Os valores registrados de RSS do compositor no Linux permaneceram iguais ou aumentaram em uma página de 4 KiB. Esses testes finitos detectam crescimento nas cargas exercitadas; não estabelecem um limite geral de memória nem medem memória da GPU física.

Três testes em [presentation.rs](../tests/cases/presentation.rs) cobrem envios rejeitados. Um proxy de teste substitui o pixmap em uma requisição Present por outro incompatível, produzindo um `BadMatch` do servidor real. Antes, o compositor encerrava. Agora ele identifica o envio exato, verifica se o buffer original e a saída ainda existem com tela e profundidade compatíveis e continua com XRender pelo restante da sessão, inclusive após redimensionar a raiz. Erros de pixmap ou janela inválidos continuam encerrando o compositor. Ausência de eventos de conclusão fora de reconfigurações de monitores, tratada abaixo, e outros erros de apresentação estão fora dessa política de recuperação.

A suíte completa agora tem 59 testes aprovados: seis unitários, três de CLI e cinquenta de integração X11. Formatação, Clippy estrito, build de release e verificações da documentação passam. Hotplug físico, configurações com vários monitores e medições em hardware ainda estavam pendentes nesse ponto; veja a sessão em hardware abaixo. A configuração automatizada usa uma saída virtual de 320×240, temporariamente 240×180, com `fade_ms = 0`, `blur_radius = 0` e cada modo de vsync.

### Etapa 3 iniciada: Xmonad em Xorg e XLibre aninhados

Os primeiros [registros de desktop](benchmarks/2026-10-03/) exercitaram Xmonad 0.18.1 com Xorg Xephyr 21.1.24 e XLibre Xephyr 25.1.9 no commit `1409a919dd0c91ccaad3fbb323df4b33628554fd`. Cobriram organização de janelas, remoção de popup, tela cheia/restauração via EWMH, retorno de workspace, ciclo de vida rápido, atualização de sobrevivente e encerramento via SIGTERM com Present ativado.

O [script de desktop](../tools/desktop-check.sh) agora aceita `present` ou `direct`. O probe observa Damage do overlay para prontidão e atualizações diretas, exige amostras Present somente nesse caminho e rejeita configurações de apresentação incompatíveis. Inicialização e cenas usam notificações com prazo máximo em vez de consultas com atrasos. Os relatórios registram alterações e hashes dos fontes junto ao commit base.

As quatro combinações de servidor/caminho passaram na base `cb0796c8e42a95d3c80ab11c557b75809f791d43` mais as alterações arquivadas do probe/script. O [guia bilíngue de validação e as evidências brutas](DESKTOP_TESTING.pt-BR.md#medição-registrada-2026-10-03) registram fases ociosa/ativa de dez segundos, CPU separada do compositor/servidor, RSS inalterado nos extremos, intervalos Present e dois testes negativos. Suíte completa de 59 testes, formatação, Clippy estrito e build de release passam. Isso conclui o incremento da medição isolada, não o requisito de hardware da etapa 3. Diagnósticos existentes do Xmonad e ambientes ainda não testados estão registrados no guia.

### Etapa 2 em hardware: hotplug físico no XLibre com AMD

Uma sessão nativa do XLibre 25.1.9 com o driver modesetting, o driver de kernel amdgpu, um AMD Ryzen 5 5600GT (gráficos Radeon Vega, Mesa 26.2.4) e Xmonad 0.18.1 executou o novo [script de transições de monitores](../tools/hotplug-check.sh). HDMI-1 (principal, à direita) e DP-1 (um adaptador DisplayPort para VGA, à esquerda) usaram 1920×1080 a 60 Hz. O [probe de desktop](../examples/desktop_probe.rs) mantém um único marcador override-redirect. Após cada transição, ele verifica atualizações perto de cantos opostos de cada monitor ativo e sobre cada borda compartilhada entre monitores; em seguida, registra a topologia RandR, fases ociosa e ativa de dez segundos e a contagem XRes do compositor.

A primeira execução encontrou um defeito. Mudar o HDMI-1 para 1280×720 congelou a imagem no modo Present: uma reconfiguração de CRTC e o redimensionamento da raiz ocorreram com um envio pendente, e o servidor nunca enviou os eventos de conclusão e ociosidade correspondentes. O Compust aguardava esses eventos antes de recriar seus buffers. Uma [reprodução instrumentada](benchmarks/2026-10-03/hardware/stall/diagnosis/compust-debug.log) registrou a ausência dos eventos. Agora uma mudança RandR substitui o renderizador sem esperar, e um `ConfigureNotify` da raiz com o mesmo tamanho não cancela mais uma substituição solicitada antes no mesmo lote de eventos. Duas regressões em [presentation.rs](../tests/cases/presentation.rs) retêm os eventos de um envio com uma fence SYNC de espera que nunca é acionada. Ambas falhavam antes da correção; o caso de desligar e religar a saída também falhava apenas com a primeira mudança.

Com a correção, os dois modos de apresentação passaram em quinze amostras: a referência inicial, o modo 1280×720 e sua restauração, uma disposição vertical de 1920×2160 e sua restauração, DP-1 desligado e religado e, para cada conector, desconexão física com o CRTC ainda atribuído, `xrandr --auto`, reconexão e restauração. Em 8.988 intervalos das fases ativas, o Present teve mediana e p95 de 16,667 ms e máximo de 16,670 ms. Com o marcador atualizado 60 vezes por segundo, o Compust usou 0,3–0,5% de um núcleo e o Xorg, 3,2–4,4%. Topologias repetidas apresentaram os mesmos bytes de pixmaps e as mesmas contagens de recursos, e o RSS do Compust ficou entre 3.760 e 3.896 KiB. Detalhes e evidências brutas estão no [guia de validação de desktops](DESKTOP_TESTING.pt-BR.md#sessão-registrada-em-hardware-2026-10-03).

A suíte completa agora tem 61 testes aprovados: seis unitários, três de CLI e cinquenta e dois de integração X11. Formatação, Clippy estrito e build de release passam. Isso valida transições de monitores somente no ambiente registrado, com fades e desfoque desativados. Drivers Intel e NVIDIA, Xorg em hardware, taxas de atualização mistas, mais de dois monitores e os cenários de desktop da etapa 3 em hardware continuam em aberto.

### Etapa 3 em hardware: cenários do Xmonad e efeitos em AMD/XLibre

O probe ganhou um cenário de papel de parede, que define e remove `_XROOTPMAP_ID` no workspace vazio, e uma opção `--opacity` para a janela medida. O novo modo `effects` do script usa os fades padrão de 180 ms e raio de desfoque 4 com uma janela 50% translúcida, e o script agora pode usar um servidor dedicado já existente. [`hardware-session.sh`](../tools/hardware-session.sh) executa os três modos como cliente de um novo servidor X iniciado em um console de texto.

Na máquina AMD/XLibre, uma sessão dedicada apenas com o HDMI-1 em 1920×1080 passou em todos os cenários dos três modos. As conclusões Present acompanharam o vblank com exatidão: 600 quadros a 16,667 ms, com o Compust em 0,3% e o Xorg em 3,1% de um núcleo. O modo de efeitos revelou o principal problema de desempenho. O desfoque atrás de uma janela translúcida em tela cheia levou 200–217 ms por quadro enquanto o Xorg usava 93,7%. O glamor acelera apenas filtragem nearest e bilinear, então o filtro de convolução usado pelo desfoque do Compust recorre à CPU a cada atualização. O Xephyr aninhado em 1280×800 mostrou o mesmo limite, 85 quadros em dez segundos. O [guia de validação de desktops](DESKTOP_TESTING.pt-BR.md#sessão-de-desktop-registrada-em-hardware-2026-10-03) traz as medições e limitações.

Isso conclui os cenários do probe em um ambiente de hardware, com Xmonad e janelas sintéticas. Aplicativos reais, gerenciadores com decoração ou reparenting, encerramento do servidor com o compositor em execução, Xorg em hardware e outras GPUs continuam em aberto. O desfoque precisa de uma implementação que o glamor possa acelerar antes de ser recomendado em drivers baseados em glamor. Formatação, Clippy estrito, a suíte de 61 testes e o build de release passam.

### Desfoque no caminho da GPU

A medição da etapa 3 mostrou que o glamor processa o filtro de convolução na CPU. O desfoque agora monta uma pirâmide: cada nível reduz pela metade a área em torno de uma janela translúcida com amostragem bilinear, e o nível mais grosso é ampliado de volta dentro do formato da janela. O glamor acelera essas transformações. O raio configurado é arredondado para 2, 4, 8 ou 16 pixels. Os buffers dos níveis só existem com o desfoque ativado e substituem um buffer auxiliar do tamanho da tela.

No desktop AMD/XLibre, o modo de efeitos passou de 49 para 599 quadros em dez segundos, com todos os intervalos Present em um vblank. O Xorg usou 4,1% de um núcleo em vez de 93,7%. O Xephyr aninhado passou de 85 para 598–599 quadros, usando cerca de 20% de um núcleo em vez de 86%. O [guia de validação de desktops](DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03) traz os registros.

Uma nova regressão desfoca uma borda preta/branca alinhada com todos os níveis nos raios 4 e 16. Ela exige uma transição monotônica centrada na borda, com pixels distantes inalterados; deslocar uma passada em um pixel faz o teste falhar. Os testes existentes de listras e de formato fora da tela passam sem mudanças, e três testes unitários cobrem o arredondamento do raio e os limites da pirâmide. A suíte completa tem 65 testes aprovados: nove unitários, três de CLI e cinquenta e três de integração X11.

### Etapa 4: primeira pré-versão beta

A versão 0.2.0-beta.1 está publicada como pré-lançamento no GitHub. O [guia da beta](BETA.pt-BR.md) declara o escopo: XLibre 25.1.9 com Xmonad 0.18.1 na máquina AMD/glamor registrada, com um ou dois monitores 1080p a 60 Hz. Ele também cobre instalação com verificação de checksums, inicialização com Xmonad ou `~/.xinitrc`, retorno ao compositor anterior, limitações conhecidas e as informações de que um relato de falha precisa. O [modelo de relato](../.github/ISSUE_TEMPLATE/bug.yml) pede os mesmos dados.

[`package.sh`](../tools/package.sh) gera uma versão a partir de uma exportação limpa de uma revisão, com a toolchain fixada. Ele remove caminhos locais do binário, registra `BUILDINFO` e grava um arquivo compactado determinístico com `SHA256SUMS`; duas execuções na máquina de compilação produziram arquivos idênticos. O binário empacotado exibiu a versão, recusou-se a iniciar ao lado de outro compositor, encerrou com sucesso após SIGTERM e saiu com erro de conexão quando seu servidor X parou. Com esse binário, as verificações de desktop aninhadas passaram nos três modos no XLibre Xephyr, e a CI passou no commit da versão.

Os registros em hardware usaram o mesmo código compilado junto com o probe de desktop, o que apenas acrescenta suporte a X-Resource ao x11rb; o binário publicado é compilado sozinho. Relatos da beta em outros ambientes definirão o que uma próxima versão poderá declarar.

### Etapas 2 e 3 em uma segunda máquina: Xorg com Intel

Um laptop com Linux Mint 22.3, Xorg 21.1.11 nativo, o driver modesetting com glamor, gráficos Intel Iris Plus G1 (i915, Mesa 25.2.8) e Xmonad 0.17.2 repetiu os procedimentos de qualificação. É o primeiro registro do Xorg em hardware e de um driver Intel. As verificações aninhadas passaram nos três modos. No desktop habitual, os dois modos de apresentação passaram em uma referência, na mudança do painel de 1366×768 para 1280×720 e na restauração, com contabilidade de recursos idêntica antes e depois. Em seguida, uma sessão dedicada passou em todos os cenários do probe nos modos Present, direto e de efeitos.

O Present seguiu a atualização de 60,06 Hz do painel, com mediana de 16,650 ms. Um intervalo em cada uma das duas execuções Present dedicadas durou dois vblanks. Com desfoque atrás de uma janela translúcida em tela cheia, o Xorg usou 4,4% de um núcleo, contra 4,3% sem ele; portanto, o desfoque em pirâmide também permanece na GPU aqui. O [guia de qualificação de desktops](DESKTOP_TESTING.pt-BR.md#sessões-registradas-em-intelxorg-2026-10-03) traz as medições e os limites.

O laptop tem uma única tela, então o hotplug físico não foi testado nele. A sessão dedicada executou a mudança do tempo limite do Present descrita acima sobre o `ffd0b13`; as outras execuções usaram esse commit sem alterações. O binário publicado da 0.2.0-beta.1 não foi testado nesta máquina, então o escopo declarado da beta não muda.

### Segunda pré-versão beta

A versão 0.2.0-beta.2 traz as duas correções encontradas depois da primeira beta, as bordas completas no Xmonad e o tempo limite do Present, e acrescenta o laptop Intel/Xorg ao escopo declarado. O [guia da beta](BETA.pt-BR.md) lista as duas combinações validadas, e as [notas da versão](releases/v0.2.0-beta.2.md) listam as mudanças.

A sessão dedicada em Intel executou o mesmo código desta versão, compilado junto com o probe de desktop; os registros em AMD são anteriores às duas correções. As sessões de monitores físicos das duas máquinas não foram repetidas para esta versão.

Duas execuções do [`package.sh`](../tools/package.sh) no laptop Intel produziram arquivos idênticos. O binário empacotado exibiu sua versão, não exige símbolos da glibc mais novos que a 2.34, recusou-se a iniciar ao lado de outro compositor, encerrou com sucesso após SIGTERM e encerrou com erro de conexão quando seu servidor X parou. Com esse binário, as verificações de desktop aninhadas passaram nos três modos no Xorg Xephyr 21.1.11.

### Etapa 3 com um segundo gerenciador de janelas: Openbox

O executor de desktop e o probe agora aceitam o Openbox, o primeiro gerenciador de janelas decorado, com reparenting e empilhamento a ser testado. `WINDOW_MANAGER=openbox` o inicia com uma configuração privada, e a opção `--layout stacking` do probe acrescenta três cenários: cada cliente fica em uma moldura com reparenting e barra de título pintada, duas janelas sobrepostas vêm para a frente conforme cada uma é ativada, e uma janela minimizada sai da tela e volta. Os cenários existentes foram ajustados para janelas que mantêm seu tamanho. As execuções com Xmonad não mudaram.

No laptop Intel/Xorg, o Openbox 3.6.1 passou nas verificações aninhadas, nas amostras de transição de monitores em um desktop habitual e em uma sessão dedicada em hardware, em todos os modos de cada uma. Nenhuma mudança no compositor foi necessária. O [guia de qualificação de desktops](DESKTOP_TESTING.pt-BR.md#sessões-registradas-com-openbox-2026-10-03) traz as medições, um comportamento de inicialização do Openbox que o probe contorna e os limites.

Isso é evidência para um gerenciador de janelas empilhadas com janelas sintéticas. Mover e redimensionar de forma interativa, os menus do gerenciador e aplicativos reais ainda não têm cenários registrados, e o escopo declarado da 0.2.0-beta.2 não muda.

### Etapa 3 com um terceiro gerenciador de janelas: i3

O executor e o probe também aceitam o i3, um gerenciador de janelas tiling que coloca os clientes em molduras com barra de título. `WINDOW_MANAGER=i3` o inicia com uma configuração privada. Duas opções do probe cobrem o que difere do Xmonad: `--frames` exige a barra de título pintada fora do layout de empilhamento, e `--workspace-anchor` mapeia uma pequena janela que o i3 envia para um segundo workspace, já que o i3 não mantém workspaces vazios sem foco.

No laptop Intel/Xorg, o i3 4.23 passou nas verificações aninhadas, nas amostras de transição de monitores em um desktop habitual e em uma sessão dedicada em hardware, em todos os modos de cada uma, de novo sem mudança no compositor. O [guia de qualificação de desktops](DESKTOP_TESTING.pt-BR.md#sessões-registradas-com-i3-2026-10-03) traz as medições e os limites. O escopo declarado da 0.2.0-beta.2 não muda.

### Etapa 2 em uma segunda máquina: hotplug físico no Xorg com Intel

Com uma tela externa de 1920×1080 em seu conector HDMI, o laptop Intel/Xorg executou o procedimento de transições de monitores nos dois modos de apresentação: mudanças de modo e de layout, a saída desligada e ligada e o cabo fisicamente desconectado e reconectado. As 22 amostras passaram, as topologias repetidas reproduziram contabilidade XRes idêntica e nenhum tempo limite do Present foi registrado. O painel e a tela externa diferem na resolução e em 0,06 Hz na taxa de atualização. O [guia de qualificação de desktops](DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-hotplug-em-intelxorg-2026-10-03) traz as medições e os limites.

Este é o segundo ambiente com evidência de hotplug físico, e o primeiro no Xorg e em um driver Intel. Ele foi executado depois da 0.2.0-beta.2, cujo guia da beta ainda lista o hotplug em Intel como não testado.

### Suspensão e retomada em Intel/Xorg

O laptop Intel/Xorg foi suspenso para a RAM uma vez em cada modo de apresentação, com amostras antes e depois. As seis amostras passaram, o ritmo do Present manteve a mediana de 16,65 ms, os bytes de pixmaps próprios do compositor ficaram idênticos do início ao fim e nenhum tempo limite do Present foi registrado. O [guia de qualificação de desktops](DESKTOP_TESTING.pt-BR.md#suspensão-e-retomada-registradas-2026-10-03) traz os registros. Junto com as trocas de terminal virtual das sessões dedicadas, isso cobre os dois casos que a entrada do tempo limite do Present acima deixou sem teste, nesta única máquina; nenhum deles produziu ali uma conclusão perdida.

### Terceira pré-versão beta

A versão 0.2.0-beta.3 traz a correção para janelas que ficavam com conteúdo antigo após um redimensionamento restaurado, defeito presente nas duas betas anteriores. Seu escopo declarado acrescenta o que o laptop Intel/Xorg registrou depois da 0.2.0-beta.2: Openbox e i3, uma tela externa com hotplug físico e suspensão e retomada. O [guia da beta](BETA.pt-BR.md) lista o escopo, e as [notas da versão](releases/v0.2.0-beta.3.md) listam as mudanças.

Esses registros são anteriores à correção e usam janelas sintéticas; a correção em si foi confirmada com um teste gravado com o cabo sob o i3 e pelo teste de regressão. Nenhuma sessão em hardware foi repetida com o binário desta versão.

Duas execuções do [`package.sh`](../tools/package.sh) no laptop Intel produziram arquivos idênticos. O binário empacotado exibiu sua versão, não exige símbolos da glibc mais novos que a 2.34, recusou-se a iniciar ao lado de outro compositor, encerrou com sucesso após SIGTERM e encerrou com erro de conexão quando seu servidor X parou. Com esse binário, as verificações de desktop aninhadas passaram nos três modos com Xmonad, Openbox e i3 no Xorg Xephyr 21.1.11, com fases de três segundos.

### Quarta pré-versão beta

A versão 0.3.0-beta.1 traz tudo o que veio depois da 0.2.0-beta.3: descoberta e recarga de configuração, regras por janela, desfoque ponderado, as mudanças no caminho de eventos e no redesenho, repintura por regiões, reaproveitamento do desfoque, oclusão e o renderizador de GPU opcional. É a primeira versão no [caminho até a 1.0](#caminho-até-a-versão). O [guia da beta](BETA.pt-BR.md) lista o escopo, e as [notas da versão](releases/v0.3.0-beta.1.md) listam as mudanças.

Seu escopo declarado cobre uma máquina. No desktop com RX 9060 XT, [sessões dedicadas](DESKTOP_TESTING.pt-BR.md#sessões-de-desktop-registradas-na-rx-9060-xt-2026-10-04) com o código do compositor desta versão passaram em todos os cenários de desktop com Xmonad, Openbox, i3 e bspwm nos três modos. Cenas de benchmark em commits anteriores rodaram ali com o dwm. O desktop com Radeon Vega e o laptop Intel foram registrados com o pintor anterior, e o laptop não está mais disponível. As regras por janela têm apenas testes automatizados; o desfoque ponderado também foi conferido visualmente nos menus do Brave. As sessões usaram uma compilação local, e não o binário empacotado.

Duas execuções do [`package.sh`](../tools/package.sh) no desktop com RX 9060 XT produziram arquivos idênticos. O binário empacotado exibiu sua versão e não exige símbolos da glibc mais novos que a 2.34; ele não se liga a nenhuma biblioteca EGL, que o renderizador de GPU carrega ao iniciar. No Xvfb 21.1.24 do Xorg, ele se recusou a iniciar ao lado de outro compositor, encerrou com sucesso após SIGTERM e encerrou com erro de conexão quando seu servidor X parou.

### Matriz de compatibilidade

| Ambiente | Cobertura verificada | Evidência / limites |
| --- | --- | --- |
| Xvfb 21.1.24 no CachyOS, 320×240×24, XRender e Present 1.2 | Clientes/molduras, propriedades, sequências rápidas, fades interrompidos, formatos, destruição com eventos pendentes e onze pontos da captura; todos os 47 testes passam | Registro de 2026-10-02. Corridas de captura usam `fade_ms = 0`, `blur_radius = 0` e vsync padrão. Os casos anteriores de fades/formatos também usam `fade_ms = 1000`, `blur_radius = 4` ou `vsync = false`, conforme descrito acima. Hierarquias criadas diretamente, sem validação de gerenciador real ou GPU. |
| Mesmo Xvfb, uma saída virtual, RandR e XRes | Redimensionamento da raiz, desativação/restauração do CRTC, envio Present rejeitado e contagem repetida de recursos; todos os 59 testes passam | Registro de 2026-10-02 (horário local). Contagens e bytes do servidor são conferidos em estados renderizados equivalentes; hotplug físico e vários monitores estão fora desta configuração virtual. |
| Xorg 21.1.11 nativo no Linux Mint 22.3, modesetting + i915, Intel Core i3-1005G1 (Iris Plus G1, Mesa 25.2.8), Xmonad 0.17.2, um painel de 1366×768 a 60 Hz | Verificações aninhadas; mudança de modo do painel e restauração nos dois modos de apresentação; cenários de desktop do probe nos modos Present, direto e de efeitos; CPU, RSS, XRes e regularidade do Present | Registrado em 2026-10-03 com janelas sintéticas. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessões-registradas-em-intelxorg-2026-10-03). O hotplug físico foi registrado depois com uma [tela externa](DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-hotplug-em-intelxorg-2026-10-03). |
| Mesmo laptop Intel/Xorg com Openbox 3.6.1 | Verificações aninhadas, mudança de modo do painel e cenários de desktop do probe em todos os modos, incluindo molduras decoradas, reempilhamento e minimização | Registrado em 2026-10-03 com janelas sintéticas. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessões-registradas-com-openbox-2026-10-03). Uma [suspensão e retomada](DESKTOP_TESTING.pt-BR.md#suspensão-e-retomada-registradas-2026-10-03) por modo de apresentação também passou. |
| Mesmo laptop Intel/Xorg com i3 4.23 | Verificações aninhadas, mudança de modo do painel e cenários de desktop do probe em todos os modos, incluindo barras de título nas molduras | Registrado em 2026-10-03 com janelas sintéticas no layout dividido padrão. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessões-registradas-com-i3-2026-10-03). |
| Sessões dedicadas do Xorg 21.1.24 na mesma máquina com Radeon Vega (radeonsi, renoir), apenas HDMI-1 em 1920×1080 a 60 Hz, glamor sem TearFree, com i3 4.25.1 e sem gerenciador de janelas | Cenários de desktop do probe com troca de papel de parede; Present, XRender direto e efeitos; sombras, foco e suspensão em tela cheia em XRender e GL; CPU, RSS e cadência do Present | Registrado em 2026-10-05 em `ed2cdb0` com janelas sintéticas. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessões-xorg-registradas-no-desktop-com-radeon-vega-2026-10-05). Hotplug físico e outros gerenciadores de janelas no Xorg não foram registrados ali. |
| Xorg com drivers NVIDIA ou outras configurações AMD | Pendente | Exige registro de servidor, gerenciador, driver, configuração e commit; uma sessão AMD não comprova outros drivers. |
| XLibre com drivers Intel/NVIDIA ou outras configurações AMD | Pendente | Exige os mesmos registros de ambiente; uma sessão AMD não comprova outros drivers. |
| Xorg Xephyr 21.1.24 + Xmonad 0.18.1, aninhado no Xvfb, 1280×800×24 | Cenários de desktop com troca de papel de parede, CPU/RSS em ociosidade e atividade, modos Present, XRender direto e efeitos, encerramento | Registros de 2026-10-03: [medição](DESKTOP_TESTING.pt-BR.md#medição-registrada-2026-10-03) com fade/desfoque desativados e [medição com efeitos](DESKTOP_TESTING.pt-BR.md#medição-registrada-com-efeitos-2026-10-03). Sem validação de monitor físico ou driver. |
| XLibre Xephyr 25.1.9 + Xmonad 0.18.1, mesma disposição virtual | Mesmos cenários e medições nos três modos | Registro de 2026-10-03; mesma configuração e limitações. Resultados do servidor aninhado não validam uma sessão XLibre em hardware. |
| XLibre 25.1.9 nativo, modesetting + amdgpu, AMD Ryzen 5 5600GT (Radeon Vega, Mesa 26.2.4), Xmonad 0.18.1, HDMI + DP para VGA em 1920×1080 a 60 Hz | Mudanças de modo, disposição e saídas; desconexão e reconexão física dos dois conectores; Present e XRender direto; CPU, RSS, XRes e ritmo Present | Registro de 2026-10-03 com fade/desfoque desativados e os aplicativos do próprio desktop em execução. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessão-registrada-em-hardware-2026-10-03). O probe de cenários de desktop não foi executado nesta sessão. |
| Sessão dedicada do XLibre 25.1.9 na mesma máquina AMD, somente HDMI-1 em 1920×1080 a 60 Hz, glamor, TearFree padrão | Cenários de desktop do probe com troca de papel de parede; Present, XRender direto e efeitos (fades, translucidez, desfoque); CPU, RSS e ritmo Present | Registro de 2026-10-03 com janelas sintéticas. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessão-de-desktop-registrada-em-hardware-2026-10-03). O desfoque por convolução caiu para cerca de cinco quadros por segundo atrás de uma janela translúcida em tela cheia; o [desfoque em pirâmide](DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03) mantém 60. |
| XLibre 25.1.9 nativo, modesetting, AMD Ryzen 5 5600X + Radeon RX 9060 XT (Navi 44, radeonsi, Mesa 26.2.4), dwm 6.8, DP-2 + HDMI-1 em 1920×1080 a 60 Hz | Saídas ligadas/desligadas e mudanças de disposição com recargas de configuração de desfoque, fades e vsync; Present e XRender direto; CPU, RSS, XRes e ritmo Present | Registro de 2026-10-03 com janelas sintéticas e os aplicativos do próprio desktop em execução. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-recarga-2026-10-03). Sem desconexão física; o probe de cenários de desktop não foi executado. |
| O mesmo desktop com RX 9060 XT, um e dois monitores | Cenas de benchmark do Compust e, depois, com repintura por regiões, reaproveitamento do desfoque, oclusão e o renderizador de GPU ao lado do XRender; CPU, carga da GPU, RSS, ritmo Present e latências de abertura e fechamento | Registros de 2026-10-03 e 2026-10-04 com janelas sintéticas: [benchmarks do Compust](DESKTOP_TESTING.pt-BR.md#cenas-de-benchmark-registradas-2026-10-03), [repintura por regiões](DESKTOP_TESTING.pt-BR.md#repintura-por-regiões-registrada-2026-10-04), [reaproveitamento do desfoque](DESKTOP_TESTING.pt-BR.md#reaproveitamento-do-desfoque-registrado-2026-10-04), [oclusão](DESKTOP_TESTING.pt-BR.md#oclusão-registrada-2026-10-04) e [renderizador de GPU](DESKTOP_TESTING.pt-BR.md#renderizador-de-gpu-registrado-2026-10-04). São anteriores ao commit da versão; a próxima linha traz os cenários de desktop. |
| Sessões dedicadas do XLibre 25.1.9 no mesmo desktop com RX 9060 XT, só a DP-2 em 1920×1080 a 60 Hz, glamor, TearFree, com Xmonad 0.18.1, Openbox 3.6.1, i3 4.25.1 e bspwm 0.9.12 | Cenários de desktop do probe com troca de papel de parede; Present, XRender direto e efeitos (fades, translucidez, desfoque); CPU, RSS e ritmo Present | Registro de 2026-10-04 com janelas sintéticas e o código do compositor da 0.3.0-beta.1. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessões-de-desktop-registradas-na-rx-9060-xt-2026-10-04). Uma primeira sessão com o bspwm perdeu seu último resultado quando a máquina travou; a segunda passou. |
| XLibre 25.1.9 na RX 9060 XT, Xmonad 0.18.1, HDMI-1 a 60 Hz e DP-2 a 180 Hz | Pixels nos cantos e na junção dos monitores, redesenho de janela gerenciada e troca da saída principal; XRender Present, XRender direto e GL Present; CPU, RSS, XRes e tempos do Present | Registro de 2026-10-04 em `a3e6f31`: [as nove amostras passaram](DESKTOP_TESTING.pt-BR.md#monitores-com-taxas-diferentes-registrados-2026-10-04). Os aplicativos continuaram desenhando; o probe pede 60 atualizações/s com teto de 120 qps, então não valida agendamento independente de saída a 180 qps nem estabilidade prolongada da memória GL. |

Reproduza as verificações com a toolchain fixada pelo repositório e o Xvfb instalado. As [execuções de CI](https://github.com/hashdefault/compust/actions/workflows/ci.yml) registram resultados para cada commit exato; inclua a revisão exibida abaixo nos relatos locais.

```sh
git rev-parse HEAD
cargo test --locked --test x11 client_lifecycle
cargo test --locked --test x11 properties
cargo test --locked --test x11 stability
cargo test --locked --test x11 fades
cargo test --locked --test x11 shapes
cargo test --locked --test x11 destruction
cargo test --locked --test x11 capture_races
cargo test --locked --test x11 monitors
cargo test --locked --test x11 resources
cargo test --locked --test x11 presentation
```

Defina `XVFB=/caminho/para/Xvfb` se o servidor estiver fora de `PATH`.

### Critérios ainda pendentes

Os cenários do probe passam em uma sessão AMD/XLibre e em uma sessão Intel/Xorg com Xmonad, no laptop Intel/Xorg com Openbox e i3, e em uma sessão AMD/Xorg com i3. Estenda os testes em hardware a mais gerenciadores de janelas, a aplicativos reais e aos drivers NVIDIA. Registre servidor, driver, configuração e commit em cada relato. Cubra mudanças de parentesco após a inicialização, sequências rápidas de map/unmap/destroy, janelas decoradas e override-redirect, menus, tela cheia, ferramentas de papel de parede e encerramento da sessão.

Repita o hotplug físico e as configurações com vários monitores com outros drivers e servidores e mais de dois monitores, além de medir memória e apresentação em execuções mais longas. [Taxas de 60/180 Hz](DESKTOP_TESTING.pt-BR.md#monitores-com-taxas-diferentes-registrados-2026-10-04) agora têm um registro curto na RX 9060 XT; a cadência independente por monitor e outros ambientes continuam sem validação. As sessões de monitores AMD/XLibre e Intel/Xorg, as transições RandR virtuais, a recuperação de envios Present rejeitados ou não concluídos e a contagem repetida via XRes acima estão concluídas. Preserve a cobertura de destruição entre requisições de captura, liberação de recursos, formatos grandes ou fora da tela e propriedades malformadas registrada acima.

**Aceitação:** reproduções documentadas viram testes quando viável; o uso normal não causa quedas nem deixa janelas invisíveis ou imagens antigas; mudanças repetidas de ciclo de vida não fazem os recursos do servidor crescerem indefinidamente. Mantenha uma matriz de compatibilidade com evidências.

## Trabalho de renderização: quatro etapas concluídas em um desktop

As quatro etapas deste marco estão concluídas no desktop com RX 9060 XT. A etapa 1 ainda não tem uma segunda máquina, que a [etapa de desempenho da 1.0](#3-medições-independentes-de-desempenho) registra. A beta mostrou onde estava o custo: o Compust repintava a tela inteira a cada evento de dano, pedia ao servidor a árvore completa de janelas a cada evento relacionado a empilhamento e repetia o desfoque para cada janela translúcida. As etapas 2 e 3 eliminaram os dois primeiros, e a etapa 4 guarda cada desfoque até que algo abaixo dele mude e pula o que janelas opacas escondem. Nas máquinas registradas, uma janela com 60 atualizações por segundo custa ao Compust menos de 2% de um núcleo e ao servidor X de 3% a 9%, e o desfoque em pirâmide acrescenta ao servidor entre um décimo de ponto e um ponto. Os [primeiros registros de benchmark](DESKTOP_TESTING.pt-BR.md#cenas-de-benchmark-registradas-2026-10-03) medem as cargas do Compust em uma máquina; nenhum registro cobre uma tela 4K, muitas janelas ou uma GPU lenta.

O marco tem quatro etapas, em ordem. As etapas 3 e 4 só começam se a etapa 1 mostrar que elas importam.

| Etapa | Estado | Resultado exigido |
| --- | --- | --- |
| 1. Cenas independentes de benchmark | Registrada no desktop com RX 9060 XT; outros equipamentos pendentes | Cenas fixas do Compust com carga, configuração, identificação do build, CPU, memória, carga da GPU e cadência dos quadros em duas máquinas. |
| 2. Idas e voltas no caminho de eventos | Concluída; uma regressão conta as requisições | Eventos de janela não custam mais uma consulta da árvore cada um; um teste conta as requisições. |
| 3. Repintura por regiões | Concluída; os quadros por região coincidem pixel a pixel com repinturas completas | Somente as regiões com dano, ampliadas para o desfoque, são repintadas e apresentadas; testes de pixels cobrem as bordas das regiões. |
| 4. Oclusão e reaproveitamento do desfoque | Concluída; cada otimização tem um teste da mudança que a desfaz | Janelas totalmente cobertas são puladas e o desfoque inalterado é reaproveitado, quando o benchmark justificar. |

### 1. Cenas independentes de benchmark

O [probe de desktop](../examples/desktop_probe.rs) mede cargas fixas do Compust: ociosidade; uma janela pequena atualizando 60 vezes por segundo; uma janela translúcida em tela cheia, com e sem desfoque; oito janelas translúcidas sobrepostas; uma cobertura opaca acima delas; movimentos e redimensionamentos; e aberturas e fechamentos repetidos. Registrar CPU do Compust e do servidor X, carga da GPU quando disponível, RSS, XRes, intervalos do Present e latências de abertura e fechamento. Identificar resolução, taxas dos monitores, configuração, pintor, duração e build exato.

**Aceitação:** duas máquinas registradas têm registros brutos de todas as cenas do Compust, o executor os reproduz, e o resumo informa custos medidos e limites de amostragem. Repetir a mesma carga e configuração ao medir variação ou uma mudança no Compust.

**Estado:** `--bench` e [`tools/bench.sh`](../tools/bench.sh) executam somente o Compust, com XRender ou seu pintor GL opcional, com e sem desfoque. Os registros na RX 9060 XT cobrem as cenas iniciais e o trabalho posterior de repintura por regiões, reaproveitamento do desfoque, oclusão e GPU. As execuções e os critérios de versão usam medições independentes do Compust. O laptop Intel/Xorg não está mais disponível, então medições atualizadas de desempenho em outra máquina continuam em aberto.

Movimentos e redimensionamentos elevaram a CPU do próprio Compust de 0,3% para 1,1% no registro inicial. O desfoque sobre oito janelas acrescentou 0,6 ponto ao Compust e 0,9 ao servidor; com a etapa 4, acrescenta cerca de 0,1 e 0,35. Essas medições das próprias cargas orientam o trabalho de renderização seguinte.

### 2. Idas e voltas no caminho de eventos

`Scene::restack` consulta os filhos da raiz a cada evento de mapeamento, reparenting, configuração e circulação, e `configure` consulta a geometria em cada um. Um redimensionamento interativo custa, portanto, várias idas e voltas por evento. Acompanhe o empilhamento pelos campos de irmão dos próprios eventos e consulte a árvore só quando a ordem for desconhecida.

**Aceitação:** uma regressão conta as requisições pelo proxy de testes existente e falha se uma rajada de eventos de configuração custar uma consulta da árvore cada; as regressões de empilhamento, destruição e corridas de captura continuam passando; a cena de mover e redimensionar da etapa 1 mostra a mudança.

**Estado:** concluída. O Compust espelha a ordem de empilhamento da raiz a partir dos eventos de estrutura e consulta a árvore apenas na inicialização ou depois de um evento citar uma janela que o espelho não conhece. Uma configuração que mantém o tamanho atualiza a posição apenas a partir do evento. A [regressão](../tests/cases/event_path.rs) move e reempilha janelas 34 vezes: isso custava 33 consultas da árvore e agora não custa nenhuma, sem consulta de geometria. Em cópias temporárias, desativar a atualização do empilhamento fez falhar ela e a regressão de empilhamento anterior, e desativar a verificação de tamanho fez falhar cinco regressões de redimensionamento. No desktop com RX 9060 XT, a cena de mover e redimensionar [custa 0,13–0,15 ponto a menos ao Compust e ao servidor X](DESKTOP_TESTING.pt-BR.md#mudança-registrada-no-caminho-de-eventos-2026-10-03). Como essa cena redimensiona a cada quadro, cada evento ainda recaptura a janela, que continua sendo o custo maior. Os 66 testes X11 passam com o Xvfb 21.1.24 do Xorg, junto com 14 testes unitários e 6 de CLI; o Xvfb do XLibre continua falhando nos sete que reconfiguram o CRTC.

### 3. Repintura por regiões

Pinte e apresente apenas o que mudou. As regiões de dano precisam crescer pela margem do desfoque onde uma janela translúcida as sobrepõe, cobrir os limites antigos e novos de uma janela movida e incluir janelas em fade. O Present precisa de uma cópia por região ou de um segundo buffer cuja idade seja conhecida.

**Aceitação:** testes de pixels cobrem dano nas bordas das regiões, sob desfoque, durante o movimento de uma janela e depois de uma mudança de monitor; o trabalho ocioso não aumenta; a cena da janela pequena da etapa 1 mostra a economia, e o registro mostra também as cenas em que ela não ajuda.

**Estado:** concluída. O Damage informa o retângulo envolvente das mudanças de cada janela, e o renderizador compara o que o último quadro mostrou de cada superfície com o quadro seguinte: movimentos, redimensionamentos, recapturas, fades, saídas e mudanças de formato ou opacidade acrescentam seus limites antigos e novos, e uma reordenação acrescenta apenas a interseção das superfícies que trocaram de lugar. A área de alcance de um desfoque que a área tocava entrava nela por inteiro, repetidamente, para que o desfoque sempre lesse uma cena atual; a etapa 4 substituiu isso por fundos guardados. A pintura é recortada pela área, o Present a recebe como região de atualização do único buffer, e a cópia via XRender é recortada da mesma forma. Os [testes de regiões](../tests/cases/regions.rs) conferem que uma atualização de 10×10 apresenta exatamente a própria área e preserva os vizinhos, que um movimento e uma reordenação apresentam apenas os limites e a interseção envolvidos e que os fades ficam dentro da janela que esmaece. Dano sob desfoque, uma cadeia de desfoques sobrepostos e uma mudança de monitor coincidem pixel a pixel com uma repintura completa, com Present e com XRender, e uma exposição do overlay, como a de um bloqueador de tela que desenha no overlay, é repintada. Em cópias temporárias, remover cada parte da mudança fez falhar ao menos um desses testes: o espalhamento do desfoque ou sua repetição, os limites antigos de um movimento, a interseção da reordenação, a comparação de opacidade, a região de atualização, os avisos de retângulo envolvente, o primeiro quadro completo e o tratamento de exposições. No desktop com RX 9060 XT, o servidor X [economizou 0,7 ponto de um núcleo](DESKTOP_TESTING.pt-BR.md#repintura-por-regiões-registrada-2026-10-04) com a janela pequena em dois monitores e 0,9 com oito janelas translúcidas, e o estado ocioso continuou ocioso; abrir e fechar mediu dois ticks de CPU a menos, dentro da resolução dessa cena curta. Uma janela translúcida em tela cheia e pilhas de janelas desfocadas repintam tanto quanto antes e custam o mesmo, e a carga da GPU caiu cerca de um ponto, não pela metade. Os 74 testes X11 passam com o Xvfb 21.1.24 do Xorg, junto com 21 testes unitários e 6 de CLI.

### 4. Oclusão e reaproveitamento do desfoque

Pule janelas totalmente cobertas por janelas opacas e sem formato acima delas, e reaproveite o fundo desfocado de uma janela enquanto nada abaixo dela mudar. As duas otimizações acrescentam estado que pode ficar desatualizado; por isso só valem a complexidade se as cenas de oito janelas e de desfoque da etapa 1 mostrarem um custo real.

**Aceitação:** cada otimização tem testes de pixels para o caso que a invalida e uma cena registrada em que economiza trabalho.

**Estado:** concluída. Só com a repintura por regiões, o desfoque sobre oito janelas sobrepostas acrescentava 0,6–0,7 ponto ao Compust e 1,2–1,9 ao servidor X no desktop com RX 9060 XT, [o maior custo restante](DESKTOP_TESTING.pt-BR.md#repintura-por-regiões-registrada-2026-10-04) dessa cena, porque uma mudança na janela de cima desfocava de novo todas as janelas abaixo dela; um terminal translúcido sob desfoque fazia o mesmo a cada tecla. Agora cada mudança registra a camada mais baixa da pilha que ela altera, e cada janela desfocada guarda seu fundo desfocado. Ela só é desfocada de novo quando uma mudança abaixo dela alcança sua área de alcance ou quando o fundo guardado não corresponde mais aos seus limites; qualquer outra repintura copia o fundo guardado.

Os [testes de fundos](../tests/cases/backdrops.rs) fazem cada tipo de mudança dentro, acima, ao lado e abaixo de uma janela desfocada, incluindo reordenações, um desmapeamento e movimentos da janela e de outra abaixo dela, e comparam cada quadro com um de um renderizador novo, com Present e com XRender. Mudanças dentro ou acima da janela apresentam apenas a própria área, uma janela desfocada que surge e some em fade é desfocada uma vez, e os buffers de fundo não vazam quando janelas são mapeadas, redimensionadas e fechadas. Em cópias temporárias, testes X11 falharam sem o novo desfoque para mudanças abaixo, sem as mudanças de cena de reordenações, saídas ou antigos vizinhos, sem fundos guardados, com um novo desfoque para as mudanças da própria janela e com fundos nunca descartados. Uma regra, a de que uma janela desfocada de novo muda a cena das janelas acima dela, falhou apenas no seu teste unitário: nas cenas testadas, a margem da área de alcance impediu que seu efeito chegasse aos pixels de cima.

No desktop com RX 9060 XT, a cena de oito janelas com desfoque [custou ao Compust 0,50% em vez de 1,02%](DESKTOP_TESTING.pt-BR.md#reaproveitamento-do-desfoque-registrado-2026-10-04) e ao servidor X 0,8–1,2 ponto a menos; o desfoque agora acrescenta cerca de 0,1 ponto ao Compust e 0,35 ao servidor em relação à mesma cena sem desfoque. Uma janela translúcida em tela cheia com desfoque custou 0,13–0,18 ponto a menos aos dois. Os fundos guardados são pixmaps do servidor, um por janela desfocada: os pixmaps do Compust cresceram 15 MB na cena de oito janelas e 25 MB na de tela cheia. Os 80 testes X11 passam com o Xvfb 21.1.24 do Xorg, junto com 23 testes unitários e 6 de CLI.

Janelas escondidas atrás de janelas opacas são puladas. Uma janela sem canal alfa e com opacidade total esconde seu formato das janelas e do fundo abaixo dela, exceto dentro da área de alcance de um desfoque que precise ser refeito acima delas, e uma janela desfocada escondida por inteiro descarta seu fundo em vez de ser desfocada. Os [testes de oclusão](../tests/cases/occlusion.rs) contam um único Composite do RENDER em um quadro de uma janela opaca em tela cheia sobre três outras. Eles comparam quadros de um renderizador novo com os de depois que uma cobertura se afasta e volta, fica translúcida e opaca, ganha um buraco e some; comparam um desfoque sob uma cobertura com a mesma cena sem ela; e descobrem uma janela desfocada escondida enquanto a cena abaixo dela mudava. Em cópias temporárias, remover o ocultamento, a exceção para desfoques ou o descarte do fundo, deixar janelas translúcidas esconderem ou ignorar formatos fez falhar algum teste X11. Em uma nova [cena de benchmark](DESKTOP_TESTING.pt-BR.md#oclusão-registrada-2026-10-04) no desktop com RX 9060 XT, oito janelas translúcidas sob uma janela opaca em tela cheia que muda a cada quadro custaram ao Compust 0,23–0,25% em vez de 0,43–0,50% e ao servidor X 0,6–1,05 ponto a menos, com ou sem desfoque. Os 84 testes X11 passam com o Xvfb 21.1.24 do Xorg, junto com 28 testes unitários e 6 de CLI.

A etapa 3 manteve o único buffer do Present: fora da região de atualização, seu conteúdo já corresponde à tela. Múltiplos buffers de apresentação com controle explícito de propriedade continuam como avaliação em aberto para a latência. Medir a latência entre entrada e exibição exige equipamento que este projeto não tem; não a informe a partir de tempos medidos por software.

## 1.0: versão estável

**Objetivo:** uma versão para a qual um usuário do picom, em hardware comum com Xorg ou XLibre, possa migrar no uso diário, cuja configuração continue válida em toda a série 1.x e cujas alegações de suporte se apoiem em registros. Estável não significa que todo driver funcione: cada ambiente que a versão nomeia tem evidências, e os demais aparecem como não testados. Este é o marco atual. Ele vem depois do marco de renderização acima, e sua etapa 3 também dá àquele marco a segunda máquina que lhe falta. Não tem data.

### O que a 1.0 promete

- **Versões:** a partir da 1.0.0, as versões seguem o [versionamento semântico](https://semver.org/lang/pt-BR/). Toda opção de configuração e de linha de comando aceita pela 1.0 continua funcionando, com o mesmo significado, em toda versão 1.x. Novas podem surgir; remover ou mudar uma espera a 2.0, depois de uma versão 1.x que avise sobre isso.
- **Robustez:** o uso comum do desktop não para o compositor nem deixa janelas paradas ou invisíveis. Isso inclui janelas que somem entre requisições, propriedades malformadas, mudanças de monitores, suspensão e retomada e um Present recusado, cada caso com um teste de regressão ou uma sessão registrada.
- **Renderizadores:** o XRender continua como padrão. O renderizador de GPU segue opcional, com retorno ao XRender, e só vira padrão quando sessões registradas em AMD, Intel e NVIDIA mostrarem que ele desenha os mesmos quadros sem gastar mais CPU.
- **Alegações:** a versão nomeia cada ambiente qualificado com seus registros e informa os custos e limites medidos do próprio Compust em cada carga.

### Etapas

| Etapa | Situação | Resultado exigido |
| --- | --- | --- |
| 1. Recursos de que usuários do picom dependem | Implementados e testados no Xvfb: regras por janela, [sombras](#etapa-1-da-10-sombras), [suspensão da composição em tela cheia](#etapa-1-da-10-suspensão-da-composição-em-tela-cheia) e [regras pelo foco](#etapa-1-da-10-regras-pelo-foco); [cenas fixas registradas em XRender e GL em AMD/XLibre](DESKTOP_TESTING.pt-BR.md#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04) | Sombras, suspensão da composição em tela cheia e regras que escolhem pelo foco, com testes de pixels nos dois pintores |
| 2. Hardware e desktops | Um desktop AMD/XLibre registrado com a 0.3.0-beta.1 em cinco gerenciadores de janelas; Radeon Vega/AMD e Intel integrado/Xorg já registrados com hotplug físico nos dois, e depois o desktop com Radeon Vega [no Xorg com i3](DESKTOP_TESTING.pt-BR.md#sessões-xorg-registradas-no-desktop-com-radeon-vega-2026-10-05); atualização dos registros na candidata e NVIDIA pendentes | Uma versão candidata registrada em AMD, Intel e NVIDIA, no Xorg e no XLibre, sob seis gerenciadores de janelas e com aplicativos reais |
| 3. Medições independentes de desempenho | Cenas do Compust registradas na RX 9060 XT antes e depois das mudanças de renderização; outros equipamentos pendentes | Registros reproduzíveis de CPU, carga da GPU, memória, recursos e cadência dos quadros do Compust em cada ambiente suportado, sem filas persistentes nem crescimento inexplicado de recursos |
| 4. Uso prolongado | Não iniciada | 10.000 ciclos automáticos de janelas e uma semana de uso diário em duas máquinas, sem falhas nem recursos crescendo |
| 5. Testes externos | Nenhum relato externo ainda | Três testadores além do mantenedor usam uma versão candidata, que depois passa duas semanas sem novo relato de falha, janela parada ou vazamento |
| 6. Distribuição e documentação | Arquivos de versão com somas de verificação | Um pacote no Arch User Repository, páginas de manual, um guia de migração do picom, solução de problemas e uma configuração revisada |

### 1. Recursos de que usuários do picom dependem

- **Sombras, [prontas](#etapa-1-da-10-sombras):** uma sombra suave sob as janelas, com opções globais de raio, deslocamento e opacidade e uma opção `shadow` para as regras. Docks e janelas de desktop não recebem sombra por padrão, nem uma janela que desenha a própria, como o menu de um navegador ou uma janela GTK com decoração do lado do cliente. Uma sombra amplia a área que sua janela muda, nunca esconde o que está abaixo dela e acompanha o fade da janela.
- **Suspensão da composição em tela cheia, [pronta](#etapa-1-da-10-suspensão-da-composição-em-tela-cheia):** uma opção desativada por padrão. Enquanto a janela do topo é opaca e cobre a raiz inteira, o Compust para de compor: as janelas desenham direto na tela e o overlay fica oculto. A composição volta assim que isso muda. Um único overlay cobre todos os monitores; por isso, uma janela que ocupa um entre vários monitores continua composta, e a documentação diz isso.
- **Regras pelo foco, [prontas](#etapa-1-da-10-regras-pelo-foco):** um seletor `focused`, a partir de `_NET_ACTIVE_WINDOW` na raiz, para que regras deixem janelas inativas translúcidas, como faz o `inactive-opacity` do picom. A documentação lista os gerenciadores de janelas que não definem essa propriedade.

**Aceitação:** testes de pixels cobrem cada recurso, nos dois pintores quando ele desenha, comparando quadros com os de um renderizador novo: extensão, deslocamento e formato da sombra; sombras de janelas que se movem, fazem fade, ficam cobertas ou são excluídas; a suspensão iniciada e encerrada ao mapear, desmapear e redimensionar a janela em tela cheia e ao empilhar uma janela translúcida acima dela, com pixels corretos depois; e o foco passando entre janelas. Os testes de repintura por regiões, reaproveitamento do desfoque e oclusão continuam passando. Cada opção nova é documentada nos dois idiomas.

### 2. Hardware e desktops

Com uma versão candidata, registrar os cenários do probe, mudanças de monitores e as atividades abaixo em:

- AMD no Xorg e no XLibre, Intel no Xorg e o driver proprietário da NVIDIA no Xorg;
- um e dois monitores, com hotplug físico em pelo menos duas dessas máquinas e dois monitores com taxas de atualização diferentes em uma;
- seis gerenciadores de janelas: Xmonad, Openbox, i3, bspwm e dwm, registrados até agora com betas, além do Xfwm4 com o próprio compositor desligado;
- aplicativos reais: um Firefox e um navegador baseado no Chromium com seus menus, um terminal, o mpv em janela e em tela cheia, um jogo ou demonstração OpenGL em tela cheia, um aplicativo GTK com decoração do lado do cliente e um aplicativo Electron;
- trocas de área de trabalho, entrada e saída de tela cheia, arrastar e soltar, um bloqueador de tela, suspensão e retomada e encerramento da sessão.

**Aceitação:** cada ambiente tem um registro com servidor, driver, gerenciador de janelas, configuração e o commit da versão candidata, e a matriz de compatibilidade só nomeia ambientes registrados. Cada falha encontrada é corrigida, com teste de regressão quando ele consegue distingui-la, ou listada como limite conhecido antes da versão. Registros de builds anteriores permanecem, mas não qualificam a 1.0.

### 3. Medições independentes de desempenho

Rodar as [cenas de benchmark do Compust](DESKTOP_TESTING.pt-BR.md#executar-as-cenas-de-benchmark) com uma versão candidata em cada ambiente proposto para suporte. Registrar carga, resolução e taxas dos monitores, pintor, efeitos, build exato, duração, CPU do Compust e do servidor X, carga da GPU quando disponível, RSS, XRes e intervalos do Present. Repetir execuções equivalentes para distinguir variação de regressão.

**Aceitação:** cada ambiente declarado tem registros reproduzíveis das próprias cargas do Compust. Dentro do limite de quadros e do caminho de apresentação documentados, as cenas ativas acompanham a carga sem fila persistente de submissões nem recuperações repetidas, e uma cena ociosa controlada não apresenta quadros. Repetições equivalentes não têm crescimento inexplicado de RSS ociosa nem de bytes de pixmaps próprios. As notas informam custos medidos, resolução da amostragem e limites conhecidos; nenhum outro compositor é critério de versão.

### 4. Uso prolongado

**Aceitação:** uma execução automática de 10.000 ciclos que abrem, redimensionam e fecham janelas, com recargas entre eles, termina com as contagens do XRes e os bytes de pixmaps próprios que tinha depois do aquecimento. Duas máquinas usam uma versão candidata como compositor diário por sete dias, amostrando a memória do Compust e os recursos do servidor X a cada dez minutos, sem falhas, sem tempos limite repetidos do Present e com a memória ociosa do último dia a até 5% da do primeiro dia após o aquecimento.

### 5. Testes externos

Publicar betas à medida que os recursos ficarem prontos; a [0.3.0-beta.1](#quarta-pré-versão-beta) começou com tudo o que veio depois da 0.2.0-beta.3. Anunciar cada versão candidata onde usuários do X11 se reúnem.

**Aceitação:** pelo menos três testadores além do mantenedor relatam, pelos modelos de issue, o uso de uma versão candidata, e essa versão passa depois duas semanas sem novo relato de falha, de janela parada ou invisível ou de vazamento. Todo relato é corrigido ou listado como limite conhecido com seu ambiente.

### 6. Distribuição e documentação

**Aceitação:** um pacote no Arch User Repository compila a tag da versão; os arquivos da versão mantêm somas de verificação e empacotamento reproduzível; páginas de manual cobrem o comando e sua configuração; um guia de migração relaciona as opções comuns do picom às do Compust e nomeia as que não têm equivalente; e um guia de solução de problemas cobre uma inicialização recusada, o retorno do Present, tearing, o retorno do renderizador de GPU e os logs. Antes da primeira versão candidata, toda opção de configuração e de linha de comando é revisada uma vez, já que a 1.0 as congela.

### Caminho até a versão

1. **0.3.0-beta.1, [publicada](#quarta-pré-versão-beta):** tudo o que veio depois da 0.2.0-beta.3, ou seja, descoberta e recarga de configuração, repintura por regiões, reaproveitamento do desfoque, oclusão, o renderizador de GPU, desfoque ponderado e regras por janela, para que os testes externos possam começar.
2. **Outras betas 0.3** à medida que os recursos da etapa 1 ficarem prontos.
3. **1.0.0-rc.1:** etapa 1 concluída e configuração revisada e congelada; as etapas 2 a 5 rodam sobre versões candidatas.
4. **1.0.0:** uma versão candidata em que a aceitação de todas as etapas vale, publicada sem mudanças.

### Fora da 1.0

Os marcos de [Cantos arredondados](#cantos-arredondados-planejados) e de [Animações de janelas](#animações-de-janelas-planejadas), múltiplos buffers do Present, sincronização explícita, gerenciamento de cores, HDR, VRR e mais de uma tela X por processo podem vir em versões 1.x que mantenham as promessas da 1.0. Compatibilidade com a configuração do picom e Wayland estão fora do escopo do projeto.

## Backend e expansão do protocolo

Avalie um backend EGL/OpenGL ou Vulkan depois de especificar importação e sincronização. A integração moderna com X11 pode envolver DMA-BUF por DRI3, negociação de modificadores, agendamento por Present e sincronização explícita. Cada parte exige implementação real e testes de driver; consultas de versão não contam como suporte.

Gerenciamento de cores, HDR, VRR, extensões específicas do XLibre e agendamento por saída exigem investigação de protocolo e hardware antes de qualquer promessa. Wayland nativo está fora deste roteiro.

**Aceitação:** o backend importa conteúdo real sem cópia rotineira para a CPU, respeita o tempo de vida dos buffers, recupera-se das falhas previstas e apresenta cobertura de drivers e medições documentadas.

**Estado:** um backend OpenGL ES está implementado, opcional com `backend = "gl"`, e [registrado em um desktop](DESKTOP_TESTING.pt-BR.md#renderizador-de-gpu-registrado-2026-10-04). O DRI3 Open escolhe a GPU do servidor, e o BuffersFromPixmap do DRI3 1.2 compartilha o buffer de fundo e os pixmaps das janelas e do papel de parede como dma-bufs que o EGL importa sem cópia, com os modificadores do próprio servidor, então nada é negociado. A sincronização implícita ordena leituras e escritas depois que cada lado envia seu trabalho, o que uma fence do SYNC antes de cada quadro força do lado do servidor, e o Present mostra o buffer de fundo como antes. As texturas vivem tanto quanto as capturas cujos pixmaps mostram, e um renderizador e o que o substitui mantêm contextos separados, o que um teste confere no dispositivo de software do Mesa; os servidores Xvfb da CI não têm DRI3, então o pintor de GPU na tela é conferido pelo [probe dedicado em hardware](DESKTOP_TESTING.pt-BR.md#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04). A falta de DRI3, um renderizador que não abre e um quadro que falha voltam ao XRender pelo resto da sessão. Com radeonsi no XLibre, o renderizador de GPU desenha os mesmos quadros que o XRender com diferença de até dois níveis de cor, com CPU total parecida, e custa mais quando janelas são redimensionadas; o Xvfb não tem DRI3 e o Xwayland se recusa a compartilhar pixmaps, então os dois voltam ao XRender. Drivers Intel e NVIDIA, a sincronização explícita por DRI3 1.4 e uma cena registrada em que o renderizador de GPU economize trabalho continuam em aberto; até lá, o XRender continua sendo o padrão.

## Uso cotidiano

O uso da beta mostrou que mudar `fade_ms` exigia reiniciar o compositor e que uma configuração só era lida quando `--config` a indicava; a [descoberta e a recarga de configuração](#uso-cotidiano-descoberta-e-recarga-de-configuração) resolvem os dois pontos. A [1.0](#10-versão-estável) exige diagnóstico mais claro e empacotamento para distribuições. [Regras por janela](#uso-cotidiano-regras-por-janela-e-desfoque-ponderado) já definem opacidade, desfoque e duração do fade por classe, tipo ou título; o marco Animações de janelas abaixo detalha a proposta existente de movimento/escala e permitiria que as regras escolhessem animações também. Reutilize esse modelo de regras em efeitos futuros. Sombras fazem parte da 1.0; o marco de [Cantos arredondados](#cantos-arredondados-planejados) abaixo pode vir depois dela, com tratamento correto de formato e dano.

**Aceitação:** o comportamento é configurável, documentado nos dois idiomas e testável, sem anunciar implicitamente compatibilidade com a configuração ou a linguagem de animação do picom.

## Animações de janelas: planejadas

**Objetivo:** generalizar o fade já implementado para um sistema de animação por janela na abertura e no fechamento: fade (opacidade), pop (escala e opacidade) e slide (translação), com curvas configuráveis, escolhidas por janela pelas regras existentes. Este marco detalha as animações citadas acima. É uma proposta posterior à primeira beta, sem versão ou data atribuída; os quatro critérios da beta permanecem iguais.

### Ponto de partida

| Área | Implementação atual e consequência para este marco |
| --- | --- |
| Fade | [animation.rs](../src/animation.rs) armazena `from`, `to`, `started` monotônico e `duration` em `Fade`. O smoothstep inteiro produz opacidade `u16`; fechar/reabrir começa no valor amostrado. O fade está implementado, incluindo duração zero e interrupções. Preserve esse comportamento. |
| Captura e fechamento | [surface.rs](../src/surface.rs) usa `CompositeNameWindowPixmap` e uma `Picture` XRender, sem importação de textura da GPU. `Surface::close` marca a superfície como não mapeada e preserva imagem, pixmap nomeado, geometria, formato e objeto Damage. [scene.rs](../src/scene.rs) a remove quando o fade termina; os destrutores de `Surface`/`Picture` liberam Damage, imagem e pixmap próprio. O pixmap nomeado já mantém o conteúdo do fechamento após a janela original desaparecer. |
| Remapeamento e configuração | `Scene::add` captura o conteúdo novo antes de substituir uma superfície em fechamento, preservando o fade amostrado e o empilhamento. [events.rs](../src/events.rs) mapeia filhos da raiz, fecha em unmap/destroy e trata mudanças de parentesco. Configure atualiza a posição imediatamente; mudanças de tamanho/borda recapturam o pixmap preservando o fade. Generalize essas transferências para o estado completo da animação. |
| Renderização | [paint.rs](../src/renderer/paint.rs) multiplica as opacidades do fade, da janela e configurada e compõe na geometria real, pelo XRender com uma máscara A8 ou, com `backend = "gl"`, pelos shaders OpenGL ES de [gpu.rs](../src/renderer/gpu.rs). Shape e [desfoque](../src/renderer/blur.rs) também usam essa geometria. Nenhum dos dois pintores tem caminho de matriz de modelo, sombras ou cantos arredondados. |
| Agendamento e dano | [compositor.rs](../src/compositor.rs) continua pintando enquanto algum fade está ativo, solicita o redesenho final e volta à espera sem redesenho contínuo. `max_fps` limita o trabalho; Present exige conclusão e liberação antes de reutilizar seu único buffer. Cada quadro repinta apenas a área que mudou, encontrada comparando o estado mostrado de cada superfície com o do quadro anterior; não há controle de idade dos buffers. |
| Configuração e metadados | [config.rs](../src/config.rs) aceita TOML estrito, com `fade_ms = 180` nas duas direções, e [rules.rs](../src/rules.rs) resolve `[[rules]]` ordenadas que comparam classe, tipo e título do cliente, guardados em cache, e definem opacidade, desfoque e `fade_ms`. Campos desconhecidos são rejeitados; SIGUSR1 recarrega o arquivo e resolve de novo as regras de todas as janelas. As regras ainda não escolhem animações. Eventos de propriedades da raiz são assinados, mas `_NET_CURRENT_DESKTOP` não é tratado. |
| Saídas e exclusões | RandR atualmente aciona a recriação dos buffers da raiz; não há cache de geometria por saída. Tooltips override-redirect não são excluídas do fade pelo tipo. A suspensão da composição em tela cheia não existe; estar em tela cheia não equivale a estar fora do redirecionamento. |

### Escopo e preparação

1. Substituir `Fade` por um estado `Anim` com tipo, início, duração, curva, direção de abertura/fechamento e transformações inicial/final. Amostrar uma vez por quadro em `Transform { opacity, scale, offset }`. Redirecionar todos os componentes a partir dos valores atuais, inclusive ao remapear durante o fechamento; não reiniciar de um extremo nem presumir que continuidade de valor também preserve velocidade.
2. Aplicar uma transformação centrada somente durante a renderização. Para ponto local `p`, origem da janela `o` e centro `c` incluindo a borda, usar `p_out = o + c + scale * (p - c) + offset`. Preservar geometria X e regiões de entrada reais. Prototipar em XRender com `SetPictureTransform` e limites/recortes de destino transformados: sua matriz de amostragem leva coordenadas do destino à origem, portanto é preciso calcular a inversa e considerar as origens de Composite. Verificar filtragem, restauração da identidade e conversões verificadas de ponto fixo antes de adicionar efeitos. Um backend de GPU não é pré-requisito. Consulte o [protocolo Render](https://xorg.freedesktop.org/archive/current/doc/renderproto/renderproto.txt).
3. Pop abre de aproximadamente `scale = 0.85`, opacidade zero, para identidade e opacidade de animação plena; o fechamento termina no estado pequeno e transparente. Oferecer `ease_out_cubic` ou `ease_out_back` na abertura e `ease_in_cubic` no fechamento. Preservar `smoothstep`; adicionar `linear`, curvas cúbicas de entrada/saída e back-out como cálculos puros. Limitar opacidade, manter escala positiva/invertível e incluir a ultrapassagem da curva back nos limites pintados. Um modelo de mola é trabalho futuro opcional.
4. Slide abre pela borda mais próxima da saída atual da janela e fecha em direção à borda selecionada; permitir top/right/bottom/left explícitos. Escolher a saída RandR ativa com maior interseção com a janela, desempatar de forma determinística e preservar essa seleção para a captura em fechamento. Tratar origens negativas, saídas sobrepostas, janelas entre saídas e mudanças de saída explicitamente. Sem geometria confiável de saída, usar fade em vez de tratar uma raiz com vários monitores como um só monitor. Menus dropdown podem usar deslocamento curto, proposto em 24 pixels, em vez do trajeto inteiro até a borda.
5. Antes da implementação, confirmar a propriedade dos recursos retidos em unmap/destroy, falha de recaptura, remapeamento e encerramento usando a cobertura existente de [fade](../tests/cases/fades.rs), [corridas de captura](../tests/cases/capture_races.rs) e [recursos](../tests/cases/resources.rs). Preservar a última captura válida e seus metadados até a conclusão; manter capturas destruídas na posição de empilhamento já definida. Reutilizar imagens e máscaras próprias em vez de nomear/copiar pixmaps a cada quadro.
6. A repintura por regiões compara o estado exibido de cada superfície entre quadros. Incluir a transformação amostrada nesse estado e usar como limites da superfície os limites transformados, incluindo a ultrapassagem das curvas back, para que cada quadro de animação repinte a união dos limites transformados anterior e atual, ampliada para o desfoque como hoje e para eventuais sombras. Preservar propriedade dos buffers Present, alternativa por XRender direto, limite de quadros e retorno à espera ociosa, incluindo o quadro final de limpeza. Confirmar esses caminhos com várias animações simultâneas e duração zero.

### Configuração e elegibilidade propostas

Ampliar o TOML existente com `[animations]` e `[[animation_rules]]` ordenadas, ou acrescentar os campos de animação às `[[rules]]` existentes. **Esses campos são uma proposta e não são aceitos pelo binário atual.** Manter `compust.example.toml` válido até a implementação. O responsável pelo projeto deve confirmar o esquema público antes de programar, seguindo o [modelo de proposta de recurso](../.github/ISSUE_TEMPLATE/feature.yml).

- Campos globais: `kind` (`none`, `fade`, `pop`, `slide`), `open_ms`/`close_ms` separados, `open_easing`/`close_easing`, `pop_scale` (padrão `0.85`), `slide_direction` (`nearest` por padrão, ou `top`, `right`, `bottom`, `left`), `slide_offset_px` opcional (ausente significa trajeto até a borda) e `suppress_workspace_switch` (padrão proposto `true`).
- Padrões de compatibilidade: fade, smoothstep nas duas direções e durações herdadas de `fade_ms` (180 ms quando ausente). As novas durações explícitas prevalecem sobre o valor antigo por direção; zero conclui imediatamente. Os exemplos opcionais de pop/slide usam 220 ms para abrir e 150 ms para fechar. A refatoração preserva exatamente o fade das janelas elegíveis; novas exclusões e supressão em trocas de área são mudanças intencionais de elegibilidade.
- Regras comparam como as `[[rules]]` existentes: metadados em cache do cliente, a classe de recurso de `WM_CLASS`, `_NET_WM_WINDOW_TYPE` e nome (`_NET_WM_NAME`, com alternativa em `WM_NAME`), pela associação cliente/moldura, com propriedades validadas e metadados preservados após destruição. A comparação é textual exata e sensível a maiúsculas, todos os seletores informados precisam ser satisfeitos, e cada campo vem da primeira regra compatível que o define. Propriedades ausentes/malformadas não satisfazem seletores, exceto que uma janela sem tipo recebe o tipo padrão da EWMH. Atualizar metadados para transições futuras sem reiniciar uma animação ativa apenas porque o título mudou.
- Excluir tooltips override-redirect e qualquer superfície realmente fora da composição; regras podem excluir outras janelas com `kind = "none"`. Essas exclusões obrigatórias prevalecem sobre regras que habilitam animações. Menus dropdown override-redirect continuam elegíveis para regras explícitas de slide curto. Não implementar suspensão da composição em tela cheia neste marco; preservar o contrato de exclusão quando esse recurso for introduzido.
- Rejeitar chaves/tipos/curvas desconhecidos, durações inválidas, escalas não finitas ou não positivas e deslocamentos fora do intervalo antes da conexão X11. As regras são recarregadas com o restante da configuração e, como `fade_ms`, valem para transições iniciadas depois.

Exemplo opcional proposto, não uma configuração atual:

```toml
[animations]
kind = "pop"
open_ms = 220
close_ms = 150
open_easing = "ease_out_cubic"
close_easing = "ease_in_cubic"
pop_scale = 0.85
suppress_workspace_switch = true

[[animation_rules]]
window_type = "dropdown_menu"
kind = "slide"
slide_direction = "top"
slide_offset_px = 24

[[animation_rules]]
wm_class = "ExampleApp"
name = "No animation"
kind = "none"
```

### Interação, efeitos e riscos

- **Entrada:** o X encaminha cliques à geometria real, não à imagem transformada. Manter a abertura proposta entre 200 e 250 ms e o fechamento próximo de 150 ms, oferecer regras de desativação e documentar a diferença. Este marco não move janelas nem sintetiza entrada.
- **Áreas de trabalho:** observar mudanças em `_NET_CURRENT_DESKTOP` na raiz e suprimir as animações individuais de abertura/fechamento associadas. Agrupar decisões de ciclo de vida nos lotes de eventos/quadros, cancelar ou concluir animações afetadas quando a área mudar e limitar o intervalo de supressão para que aberturas comuns posteriores continuem animadas. Testar a notificação da propriedade antes e depois de map/unmap, atravessando o limite de 512 eventos por lote e em trocas sucessivas rápidas. Registrar a ordem real dos eventos no Xmonad antes de definir o intervalo; contar unmaps isoladamente não é detecção confiável. Gerenciadores que não publicam a mudança exigem uma política explícita de desativação e não devem ser anunciados como cobertos. A [propriedade EWMH](https://specifications.freedesktop.org/wm/1.5/ar01s03.html) fornece o sinal, não uma fronteira genérica de transação.
- **Gerenciadores em mosaico e Configure:** as vizinhas continuam mudando de tamanho/posição imediatamente quando o gerenciador organiza uma janela nova. Somente a janela que entra/sai anima. Configure deve preservar a animação em andamento ao substituir uma imagem e não iniciar animações para movimentos/redimensionamentos comuns.
- **Efeitos:** transformar recorte Shape, bordas, alfa por pixel, máscaras de opacidade e cobertura do desfoque com a mesma geometria. O desfoque deve amostrar a cena atrás do destino animado, sem mover um trecho antigo do fundo desfocado. Sombras e cantos arredondados não existem hoje; adicioná-los é trabalho separado, e efeitos futuros devem consumir a mesma transformação e os mesmos limites. Testar ultrapassagem, formatos vazios/desconectados, janelas grandes/fora da tela e limpeza do estado reutilizável de transformação das imagens.
- **Saídas e custo:** a borda mais próxima depende da geometria da saída, não das dimensões da raiz. Resolver mudanças de topologia sem consultar janelas destruídas nem saltar para coordenadas de saídas antigas. O desfoque de tela inteira pode dominar a CPU do servidor X com animações simultâneas; medir Compust e servidor separadamente, evitar alocações/idas e voltas por quadro e validar a cadência somente para cargas e hardware registrados.

### Verificação e aceitação

Separar testes de amostragem pura de testes de pixels/protocolo em servidor real. Seguir os prazos de eventos e as regras de tempo de vida do CONTRIBUTING; não esconder erros X11 novos nem enfraquecer testes existentes. Usar os testes Xvfb atuais, o [executor isolado do Xmonad](../tools/desktop-check.sh) e sessões físicas Xorg/XLibre registradas quando necessário.

- [ ] O fade legado mantém valores smoothstep, multiplicação de opacidade, durações, extremos com duração zero e redesenho final idênticos para a mesma sequência elegível; as regressões existentes passam.
- [ ] Pop e slide abrem/fecham janelas normais corretamente, incluindo escala centrada, direções explícitas e slide curto para dropdowns.
- [ ] Fechamentos após unmap/destroy mantêm o último conteúdo capturado sem quadros pretos/vazios; os recursos são liberados ao terminar.
- [ ] Abrir e fechar imediatamente, ou remapear durante o fechamento, parte da opacidade, escala e deslocamento atuais sem salto; falha de recaptura preserva a captura anterior e sua ordem.
- [ ] Trocas de área no Xmonad não geram tempestade de animações individuais nem capturas de fechamento persistentes; aberturas normais voltam a animar depois, inclusive após trocas rápidas repetidas.
- [ ] Tooltips excluídas e janelas excluídas pelo usuário nunca animam. A proteção de superfícies fora do redirecionamento é coberta sem anunciar que a suspensão da composição em tela cheia existe.
- [ ] Regras por janela prevalecem sobre padrões globais com precedência documentada; metadados cliente/moldura, valores ausentes/malformados, configuração legada e configuração inválida são cobertos.
- [ ] Shape, alfa, bordas e desfoque permanecem alinhados e recortados corretamente durante as transformações. Sombras/cantos arredondados futuros devem cumprir o mesmo critério quando implementados.
- [ ] Saídas com resoluções diferentes, origens negativas, janelas entre saídas, escolha da borda e mudanças de saída durante abertura/fechamento funcionam nas disposições documentadas.
- [ ] Pelo menos oito aberturas/fechamentos simultâneos respeitam o orçamento de quadro declarado, sem perdas causadas pelas animações no ambiente validado; registrar taxa de atualização, `max_fps`, desfoque, CPU, intervalos Present e comparação sem animação. Testar Present e XRender direto; não inferir ausência de tearing físico a partir de tempos de software.
- [ ] Ao terminar, o redesenho cessa e a CPU volta à referência ociosa registrada; não há loop ocupado, atraso do quadro final nem animação permanentemente ativa.
- [ ] Após aquecimento e pelo menos 1.000 ciclos de abrir/fechar, cenas estabilizadas equivalentes não apresentam crescimento nas contagens XRes nem nos bytes totais de pixmaps próprios; registrar tendências de memória do processo/servidor e testar ciclos interrompidos e encerramento.
- [ ] Formatação, Clippy estrito, regressões completas, build de release e documentação bilíngue passam antes de anunciar o marco como implementado.

### Tarefas ordenadas

Cada item corresponde a uma issue revisável. Os campos seguem o modelo de recurso: **Problema ou caso de uso**, **Comportamento proposto** e **Como verificar**. O último campo é o critério de conclusão.

1. **Confirmar contratos de tempo de vida, agendamento e configuração.** **Problema ou caso de uso:** transformações acrescentam estado a caminhos de fechamento/remapeamento que já funcionam. **Comportamento proposto:** registrar propriedade dos recursos e ordem de eventos em `surface.rs`, `scene.rs`, `events.rs` e `compositor.rs`; aprovar esquema/padrões TOML, redesenho completo, política de saída/recorte e fronteira de supressão de áreas de trabalho. **Como verificar:** cada item preparatório tem decisão concreta e uma regressão existente ou proposta associada; resolver dúvidas do esquema público antes de implementar.
2. **Generalizar o fade sem mudar a imagem.** **Problema ou caso de uso:** `Fade` amostra somente opacidade. **Comportamento proposto:** adicionar `Anim`, direção/curva e amostragem de transformação com tipos definidos em `animation.rs`; migrar criação, fechamento, remapeamento, substituição por resize e limpeza da cena mantendo escala/deslocamento identidade. **Como verificar:** amostras exatas do fade anterior e todas as regressões de pixels/recursos para interrupções e duração zero passam; falha de recaptura não descarta uma superfície em fechamento.
3. **Adicionar configuração de animações e exclusões.** **Problema ou caso de uso:** nem o `fade_ms` global nem as regras existentes escolhem uma animação. **Comportamento proposto:** ampliar `config.rs` e as regras de `rules.rs`, que já comparam metadados de clientes com precedência da primeira regra compatível, com TOML estrito para tipos de animação, opções distintas por direção e exclusões; manter os padrões legados. **Como verificar:** parsing/precedência cobrem dados válidos, ausentes, malformados e conflitantes; casos de moldura/cliente e cliente destruído selecionam a política correta. Expor cada tipo novo de animação somente quando sua tarefa de renderização estiver pronta.
4. **Introduzir transformações no XRender.** **Problema ou caso de uso:** pintura/desfoque/recortes usam geometria sem transformação. **Comportamento proposto:** adicionar amostragem afim centrada, transformações inversas de imagem, limites de destino, cobertura consistente de efeitos e estado reutilizável de transformação/filtro em `picture.rs` e `renderer/{paint,blur}.rs`. **Como verificar:** identidade reproduz os pixels atuais; amostras controladas de escala/deslocamento preservam centros, formatos, bordas, transparência, desfoque e recortes extremos sem crescimento de recursos por quadro.
5. **Implementar pop e curvas selecionáveis.** **Problema ou caso de uso:** o caminho genérico precisa de um efeito completo de escala/opacidade. **Comportamento proposto:** adicionar extremos 0,85→1 do pop, amostragem cúbica/back/linear, opacidade/escala limitadas e curvas configuráveis por direção. **Como verificar:** casos puros de extremos/ultrapassagem/interrupção e cenas reais de abertura, unmap, destroy e remapeamento passam com preservação do último conteúdo.
6. **Implementar slide por saída e deslocamentos de menu.** **Problema ou caso de uso:** bordas da raiz não representam cada monitor. **Comportamento proposto:** manter geometria negociada de monitores/CRTCs RandR, atualizar em mudanças de saída/CRTC, selecionar a saída da janela e adicionar direções automáticas/explícitas, deslocamentos dropdown e alternativa documentada. **Como verificar:** casos geométricos determinísticos e capturas cobrem resoluções diferentes, origens negativas, empates, janelas entre saídas, hotplug/resize durante animação e ausência de informações RandR.
7. **Suprimir animações provocadas por troca de área.** **Problema ou caso de uso:** a transição pode parecer várias aberturas/fechamentos independentes. **Comportamento proposto:** adicionar acompanhamento da área na raiz e classificação limitada do ciclo de vida em `atoms.rs`, `events.rs` e `compositor.rs`; concluir capturas afetadas e preservar transições posteriores reais. **Como verificar:** cenários de protocolo reordenados/em lotes e uma sessão real do Xmonad passam nos casos de troca, troca rápida, animação interrompida e abertura comum posterior; registrar comportamentos de gerenciadores não cobertos.
8. **Validar desempenho, limpeza e documentação.** **Problema ou caso de uso:** correção visual sozinha não comprova ociosidade nem estabilidade de recursos. **Comportamento proposto:** ampliar `tests/cases/`, os auxiliares XRes e o programa de verificação de desktop para transformações simultâneas, ciclos repetidos e cadência medida; atualizar README, arquitetura, exemplo TOML válido e os dois roteiros quando implementado. **Como verificar:** cada item de aceitação possui evidências do commit exato e ambiente declarado; recursos apenas propostos continuam marcados como planejados até passar nas verificações.

### Decisões e trabalho posterior

O responsável pelo projeto ainda precisa confirmar o esquema proposto `[animations]`, se as opções de animação entram em `[[rules]]` ou em uma tabela `[[animation_rules]]` separada e os exemplos opcionais de 220/150 ms; a comparação textual exata com a primeira regra compatível foi confirmada para `[[rules]]`. O plano preserva o fade legado padrão de 180 ms e propõe habilitar supressão em trocas de área. Nenhuma data de versão ou novo critério da beta foi atribuído.

Animar movimento/redimensionamento de janelas existentes e transições de slide da área de trabalho inteira está fora do escopo e permanece como trabalho futuro do roteiro. Modelo de mola, sombras, cantos arredondados e suspensão da composição em tela cheia continuam separados.

## Cantos arredondados: planejados

**Objetivo:** arredondar os cantos das janelas, com um `corner_radius` global e o mesmo campo nas regras por janela, para que a janela, o desfoque atrás dela, sua sombra e o que ela esconde sigam um único contorno arredondado nos dois pintores. Os cantos arredondados [não fazem parte da 1.0](#fora-da-10); um campo de configuração novo mantém as promessas da 1.0, então este marco pode entrar em uma versão 1.x. Ele não tem versão nem data atribuída.

### Ponto de partida

| Área | Implementação atual e consequência para este marco |
| --- | --- |
| Formato | [surface.rs](../src/surface.rs) guarda o formato delimitador da janela como retângulos, borda incluída, e sabe se esse formato é o retângulo inteiro. Recortes e a área de repintura são listas de retângulos. Uma borda arredondada precisa de cobertura parcial em cada pixel, que retângulos não expressam sem serrilhado, então ela pertence a uma máscara, enquanto os recortes continuam retangulares. |
| Pintura no XRender | [paint.rs](../src/renderer/paint.rs) compõe cada superfície com `OVER` no buffer de fundo por uma imagem A8 de 1×1 repetida que guarda sua opacidade, e mostra o fundo desfocado por pesos iguais ao alfa da janela vezes essa opacidade. Nenhuma das duas máscaras tem uma cobertura que varie ao longo da janela. |
| Pintura na GPU | [gpu.rs](../src/renderer/gpu.rs) desenha pelos `draw`, `draw_masked` e `shade` do `compust-gl`, que amostram uma origem, e uma máscara quando há uma, em `(pixel de destino + deslocamento) × escala`. Os dois pintores desenham os mesmos quadros com diferença de até dois níveis de cor, e os cantos arredondados precisam manter isso. |
| Sombras | [shadow.rs](../src/renderer/shadow.rs) desfoca o retângulo da janela com três filtros de caixa inteiros, o que se separa em uma tira por eixo; um retângulo arredondado não se separa perto dos cantos. A sombra é pintada em volta dos limites da janela e nunca sob eles, então as falhas que um canto arredondado deixa dentro dos limites ficariam sem sombra. Janelas com formato não projetam sombra. |
| Oclusão | [cover.rs](../src/renderer/cover.rs) deixa uma superfície sem alfa, com opacidade total, esconder seu formato de tudo que está abaixo dela. Uma janela arredondada deixa de cobrir seus cantos, onde a cena abaixo ainda precisa ser pintada. |
| Dano | [damage.rs](../src/renderer/damage.rs) compara os limites, o formato, a imagem, a opacidade e o desfoque mostrados de cada superfície com os do quadro anterior. Os cantos ficam dentro dos limites, então um raio que uma recarga ou uma regra por foco muda só precisa entrar nesse estado. |
| Suspensão da composição em tela cheia | [compositor.rs](../src/compositor.rs) suspende a composição apenas para uma superfície no topo que seja opaca, inteiramente retangular e cubra a tela. Uma janela arredondada não é inteiramente retangular, então uma janela em tela cheia precisa continuar quadrada, ou a suspensão nunca se aplicaria a ela. O Compust não lê `_NET_WM_STATE` hoje. |
| Regras | [rules.rs](../src/rules.rs) resolve `[[rules]]` ordenadas por classe, tipo, título e foco, e define opacidade, desfoque, duração do fade e sombra. O tipo do cliente e suas margens em `_GTK_FRAME_EXTENTS` decidem se ele projeta sombra quando nenhuma regra diz o contrário; a mesma identidade pode decidir quais janelas são arredondadas. |

### Configuração

**O responsável pelo projeto confirmou este esquema na [tarefa 1](#tarefa-1-concluída-esquema-e-método-dos-cantos). Desde a [tarefa 2](#tarefa-2-concluída-opção-e-tabela-de-cobertura), o `main` aceita estes campos, mas nada desenha cantos arredondados até a tarefa 3; a 0.3.0-beta.1 os rejeita.**

- `corner_radius`: um raio global em pixels, de 0 a 64. Zero, o padrão, não arredonda nada, então uma atualização não muda nenhum desktop, como acontece com as sombras.
- O `corner_radius` de uma regra, no mesmo intervalo, define o raio das janelas que ela escolhe, seja qual for seu tipo ou suas margens; `corner_radius = 0` as mantém quadradas. Cada opção vem da primeira regra compatível que a define, como nos campos existentes, e uma recarga a resolve de novo.
- Sem uma regra, uma janela é arredondada quando projetaria sombra: seu tipo é `normal`, `dialog`, `utility`, `splash` ou `toolbar`, e ela não declara margens em `_GTK_FRAME_EXTENTS`, dentro das quais uma janela com decoração do lado do cliente desenha os próprios cantos. Docks, desktops, menus, dicas de ferramenta e notificações continuam quadrados, a menos que uma regra os arredonde.
- Uma janela cujo formato delimitador não é o retângulo inteiro mantém esse formato e não é arredondada. Uma janela cujo cliente o gerenciador de janelas marca com `_NET_WM_STATE_FULLSCREEN` continua quadrada, então a suspensão da composição em tela cheia ainda se aplica a ela, inclusive em um monitor entre vários. O Compust lê `_NET_WM_STATE` com o resto da identidade do cliente, e de novo quando ela muda.
- Na pintura, o raio fica limitado à metade do lado menor da janela, borda incluída, para que os arcos de uma janela pequena nunca se sobreponham.

Exemplo, que o `main` aceita sem ainda desenhar:

```toml
corner_radius = 8

# Terminais quadrados.
[[rules]]
wm_class = "Alacritty"
corner_radius = 0

# Notificações arredondadas, que seu tipo deixaria quadradas.
[[rules]]
window_type = "notification"
corner_radius = 12
```

### Renderização proposta

1. **Tabela de cobertura.** Para cada raio em uso, calcular a cobertura de um canto como uma tabela `r × r`: quantos de 16 × 16 pontos de amostra em cada pixel ficam dentro do quarto de círculo, contados com inteiros e escalados para 0–255, para que os dois pintores leiam os mesmos valores. Os quatro cantos são os quadrantes de um único disco `2r × 2r`. Manter um disco por raio em uso e liberá-lo quando nenhuma superfície usar esse raio.
2. **XRender, decidido na tarefa 1.** Enviar cada disco uma vez como imagem A8. Com opacidade total, ele é a própria máscara dos cantos; caso contrário, uma composição do disco pela máscara de opacidade de 1×1, que o quadro já preenche, grava a cobertura vezes a opacidade em uma imagem A8 temporária do tamanho do disco. Pintar a janela sem seus quatro quadrados de canto pela máscara de opacidade, sob um recorte que os deixa de fora, e depois cada quadrado de canto pelo seu quadrante da máscara dos cantos. Multiplicar os pesos do fundo desfocado pelos quadrantes do disco com `IN`, para que o desfoque nunca apareça nos cantos cortados; um fundo desfocado sem pesos usa a máscara dos cantos como a janela.
3. **GPU.** Enviar a tabela como textura e adicionar um desenho que a multiplique nos quadrados de canto da janela e do seu fundo desfocado; o resto da janela mantém os desenhos atuais. Uma distância calculada no shader seria mais simples, mas não coincidiria com os valores do XRender.
4. **Oclusão.** Uma superfície opaca arredondada esconde seu formato sem os quatro quadrados de canto, o que nunca esconde mais do que ela cobre. A cena abaixo dos cantos é pintada como abaixo de uma janela translúcida.
5. **Sombras.** As bordas mantêm suas tiras. Cada canto recebe um ladrilho quadrado com `corner_radius + 2 × shadow_radius` pixels de lado, do canto arredondado desfocado pelos mesmos três filtros de caixa em duas dimensões, calculado uma vez por par de raios. A sombra também preenche as falhas dos cantos dentro dos limites da janela, pelo complemento da cobertura, para que nenhum fundo apareça entre a janela e sua sombra e uma janela translúcida continue tão clara quanto sem a própria sombra.
6. **Dano, feito na tarefa 2.** Acrescentar o raio ao que um quadro mostrou de cada superfície. Uma mudança repinta os limites da superfície e, com sombra, a extensão dela.

### Interação e riscos

- **Bordas:** as bordas de janela do X, que o Xmonad desenha, fazem parte do retângulo da janela. O arredondamento corta sua borda externa enquanto a interna continua quadrada, então uma borda grossa fica irregular nos cantos. Arredondar também a borda interna exige a largura da borda na máscara e fica para depois; documentar a limitação.
- **Layouts lado a lado:** janelas encostadas nas vizinhas mostram o papel de parede ou a janela abaixo em cada canto. Uma regra com `window_type = "normal"` e `corner_radius = 0` mantém quadradas as janelas lado a lado.
- **Animações de janelas:** as [transformações planejadas](#animações-de-janelas-planejadas) precisam levar a máscara dos cantos com a janela, como levam seu formato e seu desfoque. O marco que chegar depois roda os testes de pixels do outro com a sua própria mudança.
- **Janelas translúcidas e ARGB:** a cobertura multiplica o alfa da própria janela, então a margem transparente de um menu continua transparente e sua borda arredondada continua suave.
- **Custo:** cada quadro que repinta uma superfície arredondada compõe mais quatro quadrados de canto para a janela, para seu fundo desfocado e para sua sombra. Medir a CPU do Compust e do servidor X com e sem cantos antes de declarar um custo.

### Verificação e aceitação

- [ ] Com `corner_radius = 0` e nenhuma regra que o defina, os dois pintores desenham exatamente os quadros de hoje, e todos os testes existentes passam sem mudanças.
- [ ] Testes unitários cobrem a tabela de cobertura: cheia dentro do arco, vazia fora dele, simétrica, monotônica em cada linha e coluna, e um raio limitado à metade do lado menor, inclusive em superfícies de 1×1.
- [ ] Testes de pixels X11 no XRender conferem cada canto de janelas opacas, translúcidas e ARGB, com e sem borda e durante fades, contra uma referência independente, com o interior e as bordas longe dos cantos inalterados.
- [ ] O desfoque é cortado nos cantos. A cena abaixo de uma janela opaca arredondada aparece pelos cantos depois de movimentos, mudanças de empilhamento e recargas, igual aos quadros de um renderizador novo.
- [ ] As sombras preenchem as falhas dos cantos sem escurecer uma janela translúcida, coincidem com um desfoque de referência em duas dimensões nos cantos e mantêm os valores atuais ao longo das bordas.
- [ ] As regras arredondam e deixam quadradas janelas por classe, tipo, título e foco; uma mudança de foco ou uma recarga que muda um raio repinta a área certa. Janelas com decoração do lado do cliente, com formato e em tela cheia continuam como estão, e a suspensão da composição em tela cheia ainda se aplica.
- [ ] O pintor de GPU desenha os mesmos quadros que o XRender com diferença de até dois níveis de cor, no dispositivo de software do Mesa e no probe em hardware.
- [ ] Após aquecimento e 1.000 ciclos de abrir e fechar com cantos arredondados, as contagens XRes e os bytes de pixmaps próprios voltam aos valores estabilizados; uma recarga que muda o raio libera as tabelas que nenhuma superfície usa.
- [ ] As cenas de benchmark registram a CPU do Compust e do servidor X com e sem cantos, nos dois pintores, em um desktop registrado.
- [ ] Formatação, Clippy estrito, a suíte completa, o build de release e a documentação nos dois idiomas, incluindo as tabelas de opções e de regras do README e o `compust.example.toml`, passam antes de anunciar o marco como implementado.

### Tarefas ordenadas

Cada item corresponde a uma issue revisável, com os campos do modelo de recurso: **Problema ou caso de uso**, **Comportamento proposto** e **Como verificar**. O último campo é o critério de conclusão.

1. **Confirmar o esquema e o método da máscara, [concluída](#tarefa-1-concluída-esquema-e-método-dos-cantos).** **Problema ou caso de uso:** depois da 1.0, um campo de configuração não pode mudar dentro da 1.x. **Comportamento proposto:** confirmar o nome, o intervalo e o padrão por tipo do campo, as políticas para janelas em tela cheia e com formato e o método dos cantos no XRender, depois de contar suas requisições por quadro. **Como verificar:** cada questão em aberto desta seção tem uma decisão registrada.
2. **Adicionar a opção e a tabela de cobertura, [concluída](#tarefa-2-concluída-opção-e-tabela-de-cobertura).** **Problema ou caso de uso:** nada escolhe nem calcula um raio. **Comportamento proposto:** adicionar `corner_radius` a `config.rs` e `rules.rs`, ler o estado de tela cheia do cliente com sua identidade e calcular a tabela em um novo `renderer/corner.rs`. **Como verificar:** os testes de parsing, precedência, identidade e tabela passam, e os quadros com a configuração padrão não mudam.
3. **Arredondar os cantos no XRender.** **Problema ou caso de uso:** a máscara de opacidade e os pesos do fundo desfocado são uniformes nos cantos. **Comportamento proposto:** máscaras de canto para a janela e seu fundo desfocado em `paint.rs`, e os cantos fora do que `cover.rs` esconde. **Como verificar:** os testes de pixels, desfoque, oclusão e repintura por região acima passam no Xvfb.
4. **Arredondar as sombras.** **Problema ou caso de uso:** as tiras da sombra não desenham um canto arredondado, e nada preenche as falhas. **Comportamento proposto:** ladrilhos de canto e preenchimento das falhas em `shadow.rs`, nos dois pintores. **Como verificar:** os testes de sombra acima passam, e os testes de sombra existentes passam sem mudanças.
5. **Arredondar os cantos no pintor de GPU.** **Problema ou caso de uso:** o `compust-gl` não tem um desenho que multiplique uma textura de cobertura. **Comportamento proposto:** a textura de cobertura e os desenhos de canto em `gpu.rs` e `crates/gl`. **Como verificar:** os testes de paridade passam no dispositivo de software do Mesa e no probe em hardware.
6. **Validar e documentar.** **Problema ou caso de uso:** testes de pixels sozinhos não mostram custo nem estabilidade de recursos. **Comportamento proposto:** ciclos de recursos em `tests/cases/`, execuções de benchmark em um desktop registrado e atualizações no README, na arquitetura, no `compust.example.toml` e nos dois roteiros. **Como verificar:** cada item de aceitação tem evidências do commit e do ambiente exatos.

### Tarefa 1 concluída: esquema e método dos cantos

O responsável pelo projeto confirmou em 2026-10-05: o campo é `corner_radius`, de 0 a 64, global e nas regras; sem uma regra, as janelas são arredondadas quando projetariam sombra; janelas em tela cheia, reconhecidas por `_NET_WM_STATE_FULLSCREEN`, e janelas com formato continuam quadradas.

Um programa descartável, mantido fora do repositório, desenhou uma janela de 200×150 com cantos arredondados em um buffer preto de 320×240×24 no Xvfb 21.1.24, na ordem que a etapa 2 descreve, e comparou cada pixel do buffer com valores calculados na CPU com o arredondamento do pixman. Todos os pixels coincidiram nos raios 1, 2, 8, 13 e 64, com uma janela opaca e uma ARGB semitransparente, e com as opacidades `0xff`, `0x99` e `0x01`. Montar a máscara temporária preenchendo-a com a opacidade e multiplicando o disco com `IN`, como proposto antes, desenhou os mesmos pixels com uma requisição a mais. Quatro composições `IN` dos quadrantes do disco multiplicaram uma imagem A8 de pesos pela cobertura com exatidão em todos os pixels. As tabelas foram simétricas e nunca diminuíram em direção ao interior nos raios 1, 2, 3, 8, 13 e 64; a partir do raio 4 o pixel mais externo é vazio, e o mais interno é cheio.

| Requisições de uma janela arredondada em um quadro | Hoje | Com cantos arredondados |
| --- | --- | --- |
| Com opacidade total | 3 | 8: mais um recorte e quatro composições de canto |
| Translúcida ou em fade | 3 | 9: também a máscara temporária |
| Mostrar um fundo desfocado abaixo dela | Mais 1, ou 3 com pesos | Mais 4 que hoje, ou 5 com pesos |

O programa contou as duas primeiras linhas; a linha do fundo desfocado foi contada a partir do código e da ordem da etapa 2, sem o próprio desfoque. Nenhuma dessas requisições espera resposta. O maior disco e sua imagem temporária, no raio 64, são imagens A8 de 128×128, com 16 KiB cada. As tarefas 2 e 3 transformam essas verificações em testes unitários e X11 do repositório.

### Tarefa 2 concluída: opção e tabela de cobertura

`corner_radius` é uma opção global e um campo de regra, cada um de 0 a 64 e rejeitado fora disso; o raio de uma regra, inclusive zero, conta como opção. O raio de cada superfície vem de [`rules::corner_radius`](../src/rules.rs): o da sua regra ou, na falta dele, o global quando a janela é decorada, a mesma condição que lhe dá sombra, que a identidade agora chama de `decorated`. Uma janela em tela cheia, cujo `_NET_WM_STATE` do cliente lista `_NET_WM_STATE_FULLSCREEN`, não recebe raio, digam o que disserem suas regras, nem uma janela com formato. A identidade lê `_NET_WM_STATE` como sétima propriedade na mesma ida e volta, como uma lista de no máximo 32 átomos, e de novo quando ela muda. [corner.rs](../src/renderer/corner.rs) limita o raio à metade do lado menor da superfície, borda incluída, e calcula o disco de cobertura como a tarefa 1 o mediu. O que um quadro mostrou de uma superfície inclui seu raio, então a etapa 6 foi feita aqui em vez da tarefa 3. Nada pinta com o raio ou com o disco ainda; o disco leva um `expect(dead_code)` que a tarefa 3 precisa remover, já que o atributo quebra o build assim que algo usar o disco.

Cinco testes unitários cobrem a simetria do disco, o crescimento em direção ao meio, os valores exatos nos raios 1 e 8 e a área com erro de até um dezesseis avos de pixel por pixel de raio; o limite; a precedência das regras com decoração e tela cheia; e respostas de `_NET_WM_STATE` com tipo, formato ou tamanho errados. Um teste X11 em [rules.rs](../tests/cases/rules.rs) define estados válidos, de outro tipo, de 8 bits, longos demais e vazios, cada um seguido de uma mudança de classe que o compositor precisa acompanhar, depois destrói a janela logo após um novo estado e mapeia outra; cópias temporárias que entram em pânico com um estado de 8 bits ou com qualquer lista de átomos fazem o teste falhar. O README documenta a opção quando a tarefa 3 a desenhar. Os 122 testes X11 passam com o Xvfb 21.1.24 do Xorg, junto com 40 testes unitários e 6 de CLI, Clippy estrito e o build de release.

### Decisões e trabalho posterior

Bordas internas arredondadas, um raio diferente por canto e o arredondamento de janelas que já têm formato ficam para depois.

## Opacidade de janelas ativas e inativas: planejada

**Objetivo:** tornar a opacidade das janelas ativas e inativas uma opção de primeira classe, como `active-opacity`, `inactive-opacity` e `inactive-dim` são no picom, com transições suaves em vez de saltos. As [regras pelo foco](#etapa-1-da-10-regras-pelo-foco) já permitem que uma regra com `focused = false` deixe as janelas inativas translúcidas; este marco acrescenta opções globais que deixam menus, dicas de ferramenta e docks de fora, o escurecimento como alternativa à translucidez e mudanças suaves. Ele não é exigido pela 1.0. Campos de configuração novos mantêm as promessas da 1.0, então o marco pode entrar antes ou depois dela; ele não tem versão nem data atribuída.

### Ponto de partida

| Área | Implementação atual e consequência para este marco |
| --- | --- |
| Foco | [rules.rs](../src/rules.rs) dá às regras um seletor `focused` a partir do `_NET_ACTIVE_WINDOW` da raiz. Uma mudança custa uma leitura e resolve de novo as regras das janelas que perderam e ganharam o foco; a moldura recebe o foco junto com o cliente. Sem a propriedade, toda janela conta como focada, então nada escurece: a configuração padrão do Xmonad, sem `XMonad.Hooks.EwmhDesktops`, é um desses gerenciadores de janelas. |
| Opacidade | [paint.rs](../src/renderer/paint.rs) multiplica, a cada quadro, o `_NET_WM_WINDOW_OPACITY` do aplicativo, a `opacity` da regra ou, na falta dela, a global, e o fade de abertura ou fechamento. Uma mudança de foco, uma recarga ou uma nova opacidade do aplicativo aparece, portanto, no quadro seguinte: a janela salta para a nova opacidade. |
| Animação | [animation.rs](../src/animation.rs) suaviza cada fade com smoothstep inteiro e o redireciona a partir do valor amostrado, então um fade interrompido continua contínuo; [scene.rs](../src/scene.rs) continua pintando enquanto algum fade está em andamento. Um segundo valor do mesmo tipo, para a opacidade configurada, cabe nesse modelo. |
| Elegibilidade | Uma regra com `focused = false` também corresponde a menus, dicas de ferramenta e docks, por isso o README recomenda acrescentar `window_type = "normal"`. Cada identidade já sabe se o Compust decora a janela, a condição das sombras e dos cantos arredondados, e se o cliente está em tela cheia. |
| Efeitos | O desfoque aparece atrás de uma janela abaixo da opacidade total; só uma janela com opacidade total e sem alfa esconde o que está abaixo dela; a intensidade de uma sombra multiplica a opacidade da janela; a suspensão da composição em tela cheia exige a janela do topo com opacidade total. Uma janela inativa translúcida custa, portanto, um desfoque e deixa de esconder as janelas abaixo dela, enquanto uma escurecida continua opaca. |
| Pintores | O pintor XRender compõe cada janela por uma máscara A8 de 1×1 que guarda sua opacidade, e o pintor de GPU multiplica o mesmo valor no shader. Nenhum dos dois tem uma etapa que escureça uma janela. |

### Configuração

**O responsável pelo projeto confirmou este esquema na [tarefa 1](#tarefa-1-concluída-esquema-e-semântica); o binário atual ainda não aceita estes campos.**

- `active_opacity` e `inactive_opacity`: porcentagens, de 0 a 100, que uma janela elegível usa no lugar da `opacity` global enquanto é, ou não é, a janela ativa. Ausentes, o padrão, elas mantêm a `opacity` global, então uma atualização não muda nenhum desktop. A `opacity` de uma regra continua prevalecendo sobre as duas, como prevalece sobre a global, e a opacidade do aplicativo continua multiplicando o resultado.
- `inactive_dim`: quanto mais escura uma janela inativa elegível é desenhada, como porcentagem de preto sobre ela, de 0 a 100, padrão 0. O `dim` de uma regra, no mesmo intervalo, o define para as janelas que a regra escolhe, que um seletor `focused` pode restringir.
- Janelas elegíveis são as que o Compust decora, como nas sombras e nos cantos arredondados: tipos `normal`, `dialog`, `utility`, `splash` e `toolbar` sem margens desenhadas pelo cliente. Uma janela cujo cliente está em tela cheia não é elegível, então continua opaca e a suspensão da composição em tela cheia ainda se aplica a ela. Menus, dicas de ferramenta, docks e os demais tipos contam como ativos para essas opções; uma regra ainda pode escolher qualquer um deles pelo foco.
- Toda mudança na opacidade ou no escurecimento de uma janela é suavizada ao longo da sua duração de fade, o `fade_ms` da regra ou, na falta dele, o global, com o smoothstep dos fades de abertura e fechamento, partindo do valor que ela mostra naquele momento. Isso inclui uma mudança de foco, uma recarga e um novo `_NET_WM_WINDOW_OPACITY`. Com `fade_ms = 0`, as mudanças continuam imediatas, como hoje.

Exemplo, que o binário atual ainda não aceita:

```toml
inactive_opacity = 85
inactive_dim = 10

# Terminais mantêm a aparência quando inativos.
[[rules]]
wm_class = "Alacritty"
focused = false
opacity = 100
dim = 0
```

### Comportamento proposto

1. **Opacidade suavizada.** Dar a cada superfície um valor redirecionável para sua opacidade configurada e seu escurecimento, ao lado do fade de abertura e fechamento, amostrado uma vez por quadro. Um novo alvo parte da amostra atual, então o foco que volta durante uma transição a inverte sem salto. A cena continua pintando enquanto algum desses valores se move e para quando todos se estabilizam.
2. **Opacidade ativa e inativa.** Resolver a opacidade configurada de cada superfície como a `opacity` da sua regra, senão `active_opacity` ou `inactive_opacity` quando ela é elegível, senão a `opacity` global, e redirecioná-la sempre que mudam o foco, as regras, a identidade ou a configuração.
3. **Escurecimento no XRender.** Depois de compor uma janela inativa elegível, compor preto sobre o seu recorte na intensidade do escurecimento vezes a sua opacidade, pelo próprio alfa da janela no caso de uma janela ARGB, para que uma margem transparente continue transparente. Com cantos arredondados, a máscara dos cantos vale para o escurecimento como vale para a janela.
4. **Escurecimento na GPU.** Multiplicar a cor da janela por um menos a intensidade do escurecimento no mesmo desenho, para que os dois pintores concordem com diferença de até dois níveis de cor, como hoje.
5. **Dano.** O que um quadro mostrou de uma superfície já inclui sua opacidade; acrescentar o escurecimento, para que cada passo de uma transição repinte os limites da superfície e, com sombra, a extensão dela.
6. **Foco sem `_NET_ACTIVE_WINDOW`, decidido na tarefa 1.** Com um gerenciador de janelas que não define a propriedade, acompanhar o foco de entrada do X: o cliente que o tem, ou nenhum quando o foco está na raiz ou segue o ponteiro. A propriedade continua prevalecendo onde o gerenciador de janelas a define.

### Interação e riscos

- **Custo:** com `inactive_opacity` abaixo de 100, todas as janelas menos uma ficam translúcidas: cada uma desfoca a cena abaixo dela quando o desfoque está ligado, e nenhuma esconde as janelas abaixo dela. Medir a CPU do Compust e do servidor X contra `inactive_dim`, que mantém as janelas opacas, e recomendar o escurecimento onde a translucidez custar demais.
- **Troca de área de trabalho:** o foco muda a cada troca, então as janelas da nova área se suavizam ao mesmo tempo. O número de transições é limitado pelas janelas mostradas; medir os quadros durante trocas com várias janelas.
- **Vários monitores:** uma janela inativa em outro monitor também escurece. Isso segue o picom e fica documentado, não configurável, neste marco.
- **Sombras:** a intensidade de uma sombra já acompanha a opacidade da janela, então uma janela inativa translúcida projeta uma sombra mais clara; o escurecimento deixa a sombra como está.
- **Animações de janelas e cantos arredondados:** o escurecimento precisa acompanhar as transformações do marco de [Animações de janelas](#animações-de-janelas-planejadas) e a máscara dos cantos de [Cantos arredondados](#cantos-arredondados-planejados); o que chegar depois roda os testes de pixels do outro com a sua mudança. A opacidade suavizada pode compartilhar o estado de animação generalizado que Animações de janelas propõe.
- **Foco de entrada:** ao acompanhar o foco de entrada do X, o foco na raiz, `PointerRoot`, numa moldura ou numa janela que sumiu antes de o evento de foco ser lido precisa contar como nenhum cliente ativo, sem nunca escurecer o desktop inteiro por engano.

### Verificação e aceitação

- [ ] Sem `active_opacity`, `inactive_opacity`, `inactive_dim` ou o `dim` de uma regra, os dois pintores desenham exatamente os quadros de hoje quando as transições se estabilizam, e com `fade_ms = 0` os desenham nos mesmos momentos; todos os testes existentes passam sem mudanças.
- [ ] Testes unitários cobrem a opacidade suavizada: extremos exatos, duração zero, um redirecionamento no meio que se inverte sem salto e vários alvos num mesmo quadro.
- [ ] Testes de pixels X11 nos dois pintores cobrem a opacidade ativa e inativa, a precedência com a opacidade da regra e a global, a multiplicação pela opacidade do aplicativo e a elegibilidade: menus, dicas de ferramenta, docks, janelas com decoração do lado do cliente e janelas em tela cheia continuam como estão.
- [ ] Os pixels escurecidos de janelas opacas, ARGB e com formato coincidem com uma referência independente com diferença de até dois níveis, com as margens transparentes intactas; o escurecimento acompanha os cantos arredondados se eles já existirem.
- [ ] Uma mudança de foco, uma recarga e um novo `_NET_WM_WINDOW_OPACITY` se suavizam ao longo da duração do fade, amostrados no meio da transição, e a repintura para quando eles se estabilizam, voltando à referência ociosa registrada.
- [ ] O foco acompanha o `_NET_ACTIVE_WINDOW` como hoje, e o foco de entrada do X sem ele, inclusive a raiz, `PointerRoot`, molduras e uma janela destruída antes de o evento de foco ser lido.
- [ ] Após aquecimento e 1.000 mudanças de foco, as contagens XRes e os bytes de pixmaps próprios voltam aos valores estabilizados.
- [ ] Registros em hardware com pelo menos dois gerenciadores de janelas, com o foco mudado pelo próprio gerenciador e não pelo probe, e cenas de benchmark que comparem a translucidez inativa com o escurecimento, com a CPU do Compust e do servidor X.
- [ ] Formatação, Clippy estrito, a suíte completa, o build de release e a documentação nos dois idiomas, incluindo as tabelas de opções e de regras do README e o `compust.example.toml`, passam antes de anunciar o marco como implementado.

### Tarefas ordenadas

Cada item corresponde a uma issue revisável, com os campos do modelo de recurso: **Problema ou caso de uso**, **Comportamento proposto** e **Como verificar**. O último campo é o critério de conclusão.

1. **Confirmar o esquema e a semântica, [concluída](#tarefa-1-concluída-esquema-e-semântica).** **Problema ou caso de uso:** depois da 1.0, um campo de configuração não pode mudar dentro da 1.x. **Comportamento proposto:** confirmar os nomes e intervalos dos campos, que as opções de foco substituem a opacidade global em vez de multiplicá-la, que as mudanças se suavizam ao longo da duração do fade, que janelas em tela cheia ficam de fora e se o foco de entrada do X deve ser acompanhado. **Como verificar:** cada questão em aberto desta seção tem uma decisão registrada.
2. **Suavizar as mudanças de opacidade.** **Problema ou caso de uso:** uma mudança de foco, uma recarga ou uma nova opacidade do aplicativo salta num único quadro. **Comportamento proposto:** uma opacidade configurada redirecionável por superfície em `animation.rs` e `surface.rs`, com `scene.rs` pintando até ela se estabilizar. **Como verificar:** os testes unitários e os testes de pixels no meio da transição acima passam, com os testes existentes de foco e de fade inalterados em `fade_ms = 0`.
3. **Acrescentar a opacidade ativa e inativa.** **Problema ou caso de uso:** escurecer as janelas inativas exige uma regra que precisa excluir menus, dicas de ferramenta e docks. **Comportamento proposto:** `active_opacity` e `inactive_opacity` em `config.rs`, resolvidas com elegibilidade e precedência onde `paint.rs` calcula a opacidade. **Como verificar:** os testes de parsing, precedência e elegibilidade passam, e os quadros com a configuração padrão não mudam.
4. **Escurecer as janelas inativas.** **Problema ou caso de uso:** a translucidez custa um desfoque e a oclusão de cada janela inativa. **Comportamento proposto:** `inactive_dim` e o `dim` de uma regra, desenhados em `paint.rs`, `gpu.rs` e `crates/gl`, com o escurecimento em `damage.rs`. **Como verificar:** os testes de pixels do escurecimento passam nos dois pintores.
5. **Acompanhar o foco sem `_NET_ACTIVE_WINDOW`.** **Problema ou caso de uso:** com esses gerenciadores de janelas, nada escurece. **Comportamento proposto:** o foco de entrada do X como alternativa em `events.rs` e `atoms.rs`. **Como verificar:** os testes da origem do foco acima passam, e os testes de foco existentes passam sem mudanças.
6. **Validar e documentar.** **Problema ou caso de uso:** testes de pixels sozinhos não mostram custo, o comportamento real do foco nos gerenciadores de janelas nem estabilidade de recursos. **Comportamento proposto:** ciclos de foco em `tests/cases/`, sessões em hardware com dois gerenciadores de janelas, execuções de benchmark e atualizações no README, na arquitetura, no `compust.example.toml` e nos dois roteiros. **Como verificar:** cada item de aceitação tem evidências do commit e do ambiente exatos.

### Tarefa 1 concluída: esquema e semântica

O responsável pelo projeto confirmou em 2026-10-05: os campos são `active_opacity`, `inactive_opacity` e `inactive_dim`, cada um de 0 a 100, e o `dim` de uma regra no mesmo intervalo; nas janelas elegíveis, `active_opacity` e `inactive_opacity` substituem a `opacity` global em vez de multiplicá-la, então `opacity = 90` com `inactive_opacity = 80` mostra uma janela inativa a 80%, e a `opacity` de uma regra continua prevalecendo; a opacidade e o escurecimento se suavizam ao longo do `fade_ms` da janela em vez de uma duração própria; janelas em tela cheia ficam de fora; e, sem `_NET_ACTIVE_WINDOW`, o Compust acompanha o foco de entrada do X.

### Decisões e trabalho posterior

Escurecer em direção a uma cor diferente de preto, dessaturar as janelas inativas e foco por monitor ficam para depois.

Ainda não há datas de entrega. Abra uma issue para discutir uma mudança delimitada ou relatar uma falha observada; evite iniciar vários projetos de backend sobrepostos antes de alinhar os requisitos.
