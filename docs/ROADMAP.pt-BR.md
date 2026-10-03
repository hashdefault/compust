# Roteiro de desenvolvimento

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

O objetivo é tornar este compositor mínimo em Rust uma opção prática para usuários de Xorg e XLibre. “Melhor que o picom” precisa significar melhorias observáveis em confiabilidade, latência, consumo de recursos ou manutenção. Reescrever um recurso em Rust, por si só, não comprova melhor desempenho.

## Base 0.1: implementada

O repositório contém um compositor executável com composição XRender, transições de opacidade, transparência alfa, desfoque, recorte por formato, acompanhamento de atualizações, empilhamento, tratamento de redimensionamento de janelas, propriedades de papel de parede e apresentação opcional por cópia com Present. A suíte verifica o comportamento de um servidor real no Xvfb. A documentação e as orientações de contribuição estão em inglês e pt-BR.

Esse marco cria uma base para experimentação. Ele não comprova compatibilidade completa com ambientes gráficos nem desempenho em hardware real.

## Primeira beta: quatro etapas

A primeira beta usa o backend XRender e declara suporte somente aos ambientes com evidências registradas de teste. Ela é voltada a testes controlados da comunidade. A versão 0.2.0-beta.1 foi essa beta e a 0.2.0-beta.2 vem em seguida; o [guia da beta](BETA.pt-BR.md) lista o escopo declarado atual.

| Etapa | Estado | Resultado necessário |
| --- | --- | --- |
| 1. Estabilidade das janelas | Concluída no Xvfb | Ciclo de vida, menus, transições de tela cheia e propriedades inválidas com cobertura reproduzível, sem quedas nem janelas invisíveis ou imagens antigas. |
| 2. Monitores e recursos | Verificada no Xvfb e em um desktop AMD/XLibre, além de uma mudança de modo do painel em um laptop Intel/Xorg; demais hardwares pendentes | Mudanças de resolução, conexão/desconexão de monitores, recuperação da apresentação e consumo de recursos em redimensionamentos repetidos verificados. |
| 3. Desktops reais | Cenários do Xmonad registrados em servidores aninhados, em um desktop AMD/XLibre e em um laptop Intel/Xorg; cenários do Openbox nesse laptop; outros gerenciadores e drivers pendentes | Sessões Xorg/XLibre com registro de gerenciadores e drivers testados, além de medições de CPU, memória e regularidade dos quadros. |
| 4. Distribuição da beta | Publicada como v0.2.0-beta.1; v0.2.0-beta.2 preparada com escopo declarado maior | Pré-lançamento versionado com instruções de instalação e execução, limitações conhecidas, artefatos verificados e procedimento reproduzível para relatar falhas. |

### 1. Estabilidade das janelas

Exercitar sequências rápidas de map/unmap/destroy, fades interrompidos, janelas decoradas, menus override-redirect, entrada e saída de tela cheia, propriedades malformadas e destruição durante requisições do protocolo. Começar por regressões determinísticas de pixels no Xvfb; registrar o comportamento de gerenciadores reais na etapa 3.

**Aceitação:** cada defeito reproduzido possui uma regressão que falha sem a correção; os cenários cobertos preservam os pixels corretos e mantêm o compositor em execução; suíte completa, formatação, Clippy e build de release passam. A cobertura abaixo inclui clientes e molduras, propriedades, sequências rápidas, fades interrompidos, formatos extremos ou fora da tela e destruição antes e entre requisições de captura. A validação automatizada no Xvfb está concluída; a validação de gerenciadores e drivers reais continua na etapa 3.

### 2. Monitores e recursos

Testar mudanças de resolução via RandR e hotplug físico, recuperação das falhas de apresentação previstas e redimensionamentos repetidos com contagem de recursos do servidor X. Registrar a configuração e a disposição dos monitores utilizadas.

**Aceitação:** a imagem se recupera após cada transição coberta, a apresentação continua e as operações repetidas não provocam crescimento indefinido de memória ou recursos do servidor.

Os cenários com Xvfb descritos abaixo passam. Uma sessão XLibre nativa em hardware AMD também passou na desconexão e reconexão física dos dois conectores e em mudanças de modo e disposição com dois monitores, nos dois modos de apresentação. Outros drivers e servidores, taxas de atualização mistas e mais de dois monitores continuam sem verificação; desativar um CRTC virtual não comprova o comportamento físico.

### 3. Desktops reais

Executar cenários documentados com gerenciadores de janelas reais em Xorg e XLibre. Registrar servidor, gerenciador, GPU/driver, configuração e commit exato. Medir CPU ociosa e em atividade, memória e regularidade dos quadros; corrigir falhas nos ambientes propostos para suporte na beta.

**Aceitação:** publicar uma matriz de compatibilidade com evidências para cada ambiente anunciado, uma referência reproduzível de medições e as limitações restantes. Combinações de servidor e driver ainda não testadas permanecem sem validação.

O [guia de validação de desktops](DESKTOP_TESTING.pt-BR.md) documenta o script isolado, as medições e o procedimento em hardware. As execuções registradas de Xephyr/Xmonad iniciam esta etapa; elas não concluem os requisitos de drivers e monitores físicos. Uma sessão AMD/XLibre dedicada agora executa em hardware os cenários do probe, a troca de papel de parede, os fades e o desfoque com translucidez. Aplicativos reais, gerenciadores com decoração ou reparenting, Xorg em hardware e outros drivers continuam em aberto.

### 4. Distribuição da beta

Publicar uma prévia beta versionada com instruções de compilação ou instalação do binário, exemplos de configuração, orientações de início e encerramento, checksums dos artefatos distribuídos, limitações conhecidas e um modelo de relato com diagnóstico e passos de reprodução.

**Aceitação:** uma pessoa consegue instalar e executar a versão exata, retornar ao compositor anterior e relatar uma falha seguindo as instruções fornecidas. O CI passa para o commit da versão, e as três etapas anteriores estão aprovadas dentro do escopo de suporte declarado.

A versão 0.2.0-beta.1 concluiu esses quatro critérios de liberação para seu escopo declarado. A expansão de backend de GPU e os efeitos avançados podem vir depois da primeira beta.

## Progresso e verificação

### Teste local da beta: bordas completas no Xmonad

O uso diário após `v0.2.0-beta.1` revelou bordas direitas e inferiores ausentes. Em uma janela sem formato delimitador definido pelo cliente, `ShapeGetRectangles` retornava dimensões menores que o pixmap capturado por uma largura de borda. O Compust agora usa os limites completos do pixmap nessas janelas e preserva os formatos explícitos do cliente.

A [regressão de bordas fora da tela](../tests/cases/shapes.rs) existente falhou antes da correção e passa depois. As [regressões de bordas](../tests/cases/borders.rs) cobrem mudanças de cor por foco, redimensionamento com mudança da largura da borda e remoção de formato personalizado. Todos os 68 testes, formatação, Clippy estrito e build de release passam. Na sessão local do Xmonad, as duas janelas do Alacritty mantêm as quatro faixas de borda de 2 pixels com e sem foco. Isso continua os testes da beta; as tarefas de animação abaixo permanecem planejadas.

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

### Suspensão e retomada em Intel/Xorg

O laptop Intel/Xorg foi suspenso para a RAM uma vez em cada modo de apresentação, com amostras antes e depois. As seis amostras passaram, o ritmo do Present manteve a mediana de 16,65 ms, os bytes de pixmaps próprios do compositor ficaram idênticos do início ao fim e nenhum tempo limite do Present foi registrado. O [guia de qualificação de desktops](DESKTOP_TESTING.pt-BR.md#suspensão-e-retomada-registradas-2026-10-03) traz os registros. Junto com as trocas de terminal virtual das sessões dedicadas, isso cobre os dois casos que a entrada do tempo limite do Present acima deixou sem teste, nesta única máquina; nenhum deles produziu ali uma conclusão perdida.

### Matriz de compatibilidade

| Ambiente | Cobertura verificada | Evidência / limites |
| --- | --- | --- |
| Xvfb 21.1.24 no CachyOS, 320×240×24, XRender e Present 1.2 | Clientes/molduras, propriedades, sequências rápidas, fades interrompidos, formatos, destruição com eventos pendentes e onze pontos da captura; todos os 47 testes passam | Registro de 2026-10-02. Corridas de captura usam `fade_ms = 0`, `blur_radius = 0` e vsync padrão. Os casos anteriores de fades/formatos também usam `fade_ms = 1000`, `blur_radius = 4` ou `vsync = false`, conforme descrito acima. Hierarquias criadas diretamente, sem validação de gerenciador real ou GPU. |
| Mesmo Xvfb, uma saída virtual, RandR e XRes | Redimensionamento da raiz, desativação/restauração do CRTC, envio Present rejeitado e contagem repetida de recursos; todos os 59 testes passam | Registro de 2026-10-02 (horário local). Contagens e bytes do servidor são conferidos em estados renderizados equivalentes; hotplug físico e vários monitores estão fora desta configuração virtual. |
| Xorg 21.1.11 nativo no Linux Mint 22.3, modesetting + i915, Intel Core i3-1005G1 (Iris Plus G1, Mesa 25.2.8), Xmonad 0.17.2, um painel de 1366×768 a 60 Hz | Verificações aninhadas; mudança de modo do painel e restauração nos dois modos de apresentação; cenários de desktop do probe nos modos Present, direto e de efeitos; CPU, RSS, XRes e regularidade do Present | Registrado em 2026-10-03 com janelas sintéticas. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessões-registradas-em-intelxorg-2026-10-03). Sem hotplug físico: a máquina tem uma única tela. |
| Mesmo laptop Intel/Xorg com Openbox 3.6.1 | Verificações aninhadas, mudança de modo do painel e cenários de desktop do probe em todos os modos, incluindo molduras decoradas, reempilhamento e minimização | Registrado em 2026-10-03 com janelas sintéticas. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessões-registradas-com-openbox-2026-10-03). Uma [suspensão e retomada](DESKTOP_TESTING.pt-BR.md#suspensão-e-retomada-registradas-2026-10-03) por modo de apresentação também passou. |
| Xorg com gerenciador de janelas real e drivers AMD/NVIDIA | Pendente | Exige registro de servidor, gerenciador, driver, configuração e commit. |
| XLibre com drivers Intel/NVIDIA ou outras configurações AMD | Pendente | Exige os mesmos registros de ambiente; uma sessão AMD não comprova outros drivers. |
| Xorg Xephyr 21.1.24 + Xmonad 0.18.1, aninhado no Xvfb, 1280×800×24 | Cenários de desktop com troca de papel de parede, CPU/RSS em ociosidade e atividade, modos Present, XRender direto e efeitos, encerramento | Registros de 2026-10-03: [medição](DESKTOP_TESTING.pt-BR.md#medição-registrada-2026-10-03) com fade/desfoque desativados e [medição com efeitos](DESKTOP_TESTING.pt-BR.md#medição-registrada-com-efeitos-2026-10-03). Sem validação de monitor físico ou driver. |
| XLibre Xephyr 25.1.9 + Xmonad 0.18.1, mesma disposição virtual | Mesmos cenários e medições nos três modos | Registro de 2026-10-03; mesma configuração e limitações. Resultados do servidor aninhado não validam uma sessão XLibre em hardware. |
| XLibre 25.1.9 nativo, modesetting + amdgpu, AMD Ryzen 5 5600GT (Radeon Vega, Mesa 26.2.4), Xmonad 0.18.1, HDMI + DP para VGA em 1920×1080 a 60 Hz | Mudanças de modo, disposição e saídas; desconexão e reconexão física dos dois conectores; Present e XRender direto; CPU, RSS, XRes e ritmo Present | Registro de 2026-10-03 com fade/desfoque desativados e os aplicativos do próprio desktop em execução. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessão-registrada-em-hardware-2026-10-03). O probe de cenários de desktop não foi executado nesta sessão. |
| Sessão dedicada do XLibre 25.1.9 na mesma máquina AMD, somente HDMI-1 em 1920×1080 a 60 Hz, glamor, TearFree padrão | Cenários de desktop do probe com troca de papel de parede; Present, XRender direto e efeitos (fades, translucidez, desfoque); CPU, RSS e ritmo Present | Registro de 2026-10-03 com janelas sintéticas. [Resultados e limitações](DESKTOP_TESTING.pt-BR.md#sessão-de-desktop-registrada-em-hardware-2026-10-03). O desfoque por convolução caiu para cerca de cinco quadros por segundo atrás de uma janela translúcida em tela cheia; o [desfoque em pirâmide](DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03) mantém 60. |

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

Os cenários do probe passam em uma sessão AMD/XLibre e em uma sessão Intel/Xorg com Xmonad, e no laptop Intel/Xorg com Openbox. Estenda os testes em hardware a mais gerenciadores de janelas, a aplicativos reais, aos drivers NVIDIA e ao Xorg com AMD. Registre servidor, driver, configuração e commit em cada relato. Cubra mudanças de parentesco após a inicialização, sequências rápidas de map/unmap/destroy, janelas decoradas e override-redirect, menus, tela cheia, ferramentas de papel de parede e encerramento da sessão.

Repita o hotplug físico e as configurações com vários monitores com outros drivers e servidores, taxas de atualização mistas e mais de dois monitores, além de medir memória e apresentação em execuções mais longas. A sessão de monitores AMD/XLibre, as transições RandR virtuais, a recuperação de envios Present rejeitados ou não concluídos e a contagem repetida via XRes acima estão concluídas. Preserve a cobertura de destruição entre requisições de captura, liberação de recursos, formatos grandes ou fora da tela e propriedades malformadas registrada acima.

**Aceitação:** reproduções documentadas viram testes quando viável; o uso normal não causa quedas nem deixa janelas invisíveis ou imagens antigas; mudanças repetidas de ciclo de vida não fazem os recursos do servidor crescerem indefinidamente. Mantenha uma matriz de compatibilidade com evidências.

## Depois: medir e reduzir o trabalho de renderização

As primeiras medições em hardware definiram a prioridade: o desfoque por convolução limitou um desktop AMD/XLibre a cerca de cinco quadros por segundo. A pirâmide bilinear que o substituiu mantém 60 nesse desktop, com o Xorg perto de 4% de um núcleo. Meça otimizações futuras em relação a esses registros.

Colete referências em builds de release para CPU ociosa, CPU do aplicativo e do servidor X, memória, regularidade dos quadros e latência entre entrada e exibição. Compare cenas e efeitos equivalentes com uma versão/backend registrados do picom. Inclua alta resolução e monitores com taxas diferentes.

Implemente regiões de dano, expansão das regiões de desfoque, descarte de áreas ocultas e cache somente quando a medição justificar a complexidade. Avalie múltiplos buffers de apresentação com controle explícito de propriedade e conclusão.

**Aceitação:** um benchmark reproduzível demonstra a melhora, os testes de pixels continuam corretos nas bordas das regiões e o trabalho ocioso não aumenta. Publique o benefício e as cargas em que ele deixa de existir.

## Backend e expansão do protocolo

Avalie um backend EGL/OpenGL ou Vulkan depois de especificar importação e sincronização. A integração moderna com X11 pode envolver DMA-BUF por DRI3, negociação de modificadores, agendamento por Present e sincronização explícita. Cada parte exige implementação real e testes de driver; consultas de versão não contam como suporte.

Gerenciamento de cores, HDR, VRR, extensões específicas do XLibre e agendamento por saída exigem investigação de protocolo e hardware antes de qualquer promessa. Wayland nativo está fora deste roteiro.

**Aceitação:** o backend importa conteúdo real sem cópia rotineira para a CPU, respeita o tempo de vida dos buffers, recupera-se das falhas previstas e apresenta cobertura de drivers e medições documentadas.

## Uso cotidiano

Adicione descoberta e recarga de configuração, diagnóstico mais claro e empacotamento para distribuições. O marco Animações de janelas abaixo detalha a proposta existente de movimento/escala e introduz as primeiras regras por janela com tipos definidos. Reutilize esse modelo de regras em efeitos futuros. Considere sombras e cantos arredondados com tratamento correto de formato e dano.

**Aceitação:** o comportamento é configurável, documentado nos dois idiomas e testável, sem anunciar implicitamente compatibilidade com a configuração ou a linguagem de animação do picom.

## Animações de janelas: planejadas

**Objetivo:** generalizar o fade já implementado para um sistema de animação por janela na abertura e no fechamento: fade (opacidade), pop (escala e opacidade) e slide (translação), com curvas configuráveis e regras por janela. Este marco detalha as animações e regras citadas acima. É uma proposta posterior à primeira beta, sem versão ou data atribuída; os quatro critérios da beta permanecem iguais.

### Ponto de partida

| Área | Implementação atual e consequência para este marco |
| --- | --- |
| Fade | [animation.rs](../src/animation.rs) armazena `from`, `to`, `started` monotônico e `duration` em `Fade`. O smoothstep inteiro produz opacidade `u16`; fechar/reabrir começa no valor amostrado. O fade está implementado, incluindo duração zero e interrupções. Preserve esse comportamento. |
| Captura e fechamento | [surface.rs](../src/surface.rs) usa `CompositeNameWindowPixmap` e uma `Picture` XRender, sem importação de textura da GPU. `Surface::close` marca a superfície como não mapeada e preserva imagem, pixmap nomeado, geometria, formato e objeto Damage. [scene.rs](../src/scene.rs) a remove quando o fade termina; os destrutores de `Surface`/`Picture` liberam Damage, imagem e pixmap próprio. O pixmap nomeado já mantém o conteúdo do fechamento após a janela original desaparecer. |
| Remapeamento e configuração | `Scene::add` captura o conteúdo novo antes de substituir uma superfície em fechamento, preservando o fade amostrado e o empilhamento. [events.rs](../src/events.rs) mapeia filhos da raiz, fecha em unmap/destroy e trata mudanças de parentesco. Configure atualiza a posição imediatamente; mudanças de tamanho/borda recapturam o pixmap preservando o fade. Generalize essas transferências para o estado completo da animação. |
| Renderização | [paint.rs](../src/renderer/paint.rs) multiplica as opacidades do fade, da janela e global por uma máscara A8 e compõe na geometria real. Shape e [desfoque](../src/renderer/blur.rs) também usam essa geometria. Não há shader, buffer de vértices, caminho de matriz de modelo, sombras ou cantos arredondados implementados. |
| Agendamento e dano | [compositor.rs](../src/compositor.rs) continua pintando enquanto algum fade está ativo, solicita o redesenho final e volta à espera sem redesenho contínuo. `max_fps` limita o trabalho; Present exige conclusão e liberação antes de reutilizar seu único buffer. Damage é reconhecido e marca a cena inteira para redesenho. Não há redesenho parcial nem controle de idade dos buffers. |
| Configuração e metadados | [config.rs](../src/config.rs) aceita TOML estrito, com `fade_ms = 180` nas duas direções. Campos desconhecidos são rejeitados; não há motor de regras nem recarga. A descoberta de clientes por `WM_STATE` existe, mas não há seleção de animação por classe, título e tipo. Eventos de propriedades da raiz são assinados, mas `_NET_CURRENT_DESKTOP` não é tratado. |
| Saídas e exclusões | RandR atualmente aciona a recriação dos buffers da raiz; não há cache de geometria por saída. Tooltips override-redirect não são excluídas do fade pelo tipo. A suspensão da composição em tela cheia não existe; estar em tela cheia não equivale a estar fora do redirecionamento. |

### Escopo e preparação

1. Substituir `Fade` por um estado `Anim` com tipo, início, duração, curva, direção de abertura/fechamento e transformações inicial/final. Amostrar uma vez por quadro em `Transform { opacity, scale, offset }`. Redirecionar todos os componentes a partir dos valores atuais, inclusive ao remapear durante o fechamento; não reiniciar de um extremo nem presumir que continuidade de valor também preserve velocidade.
2. Aplicar uma transformação centrada somente durante a renderização. Para ponto local `p`, origem da janela `o` e centro `c` incluindo a borda, usar `p_out = o + c + scale * (p - c) + offset`. Preservar geometria X e regiões de entrada reais. Prototipar em XRender com `SetPictureTransform` e limites/recortes de destino transformados: sua matriz de amostragem leva coordenadas do destino à origem, portanto é preciso calcular a inversa e considerar as origens de Composite. Verificar filtragem, restauração da identidade e conversões verificadas de ponto fixo antes de adicionar efeitos. Um backend de GPU não é pré-requisito. Consulte o [protocolo Render](https://xorg.freedesktop.org/archive/current/doc/renderproto/renderproto.txt).
3. Pop abre de aproximadamente `scale = 0.85`, opacidade zero, para identidade e opacidade de animação plena; o fechamento termina no estado pequeno e transparente. Oferecer `ease_out_cubic` ou `ease_out_back` na abertura e `ease_in_cubic` no fechamento. Preservar `smoothstep`; adicionar `linear`, curvas cúbicas de entrada/saída e back-out como cálculos puros. Limitar opacidade, manter escala positiva/invertível e incluir a ultrapassagem da curva back nos limites pintados. Um modelo de mola é trabalho futuro opcional.
4. Slide abre pela borda mais próxima da saída atual da janela e fecha em direção à borda selecionada; permitir top/right/bottom/left explícitos. Escolher a saída RandR ativa com maior interseção com a janela, desempatar de forma determinística e preservar essa seleção para a captura em fechamento. Tratar origens negativas, saídas sobrepostas, janelas entre saídas e mudanças de saída explicitamente. Sem geometria confiável de saída, usar fade em vez de tratar uma raiz com vários monitores como um só monitor. Menus dropdown podem usar deslocamento curto, proposto em 24 pixels, em vez do trajeto inteiro até a borda.
5. Antes da implementação, confirmar a propriedade dos recursos retidos em unmap/destroy, falha de recaptura, remapeamento e encerramento usando a cobertura existente de [fade](../tests/cases/fades.rs), [corridas de captura](../tests/cases/capture_races.rs) e [recursos](../tests/cases/resources.rs). Preservar a última captura válida e seus metadados até a conclusão; manter capturas destruídas na posição de empilhamento já definida. Reutilizar imagens e máscaras próprias em vez de nomear/copiar pixmaps a cada quadro.
6. Manter redesenho da tela inteira durante as animações neste marco, incluindo o quadro final de limpeza. Preservar propriedade dos buffers Present, alternativa por XRender direto, limite de quadros e retorno à espera ociosa. Confirmar esses caminhos com várias animações simultâneas e duração zero. Redesenho parcial continua no roteiro de desempenho; uma implementação posterior por regiões deve invalidar a união dos limites transformados anterior/atual, extensões de filtro/desfoque, ultrapassagem e eventual extensão de sombras.

### Configuração e elegibilidade propostas

Ampliar o TOML existente com `[animations]` e `[[animation_rules]]` ordenadas. **Esses campos são uma proposta e não são aceitos pelo binário atual.** Manter `compust.example.toml` válido até a implementação. O responsável pelo projeto deve confirmar o esquema público antes de programar, seguindo o [modelo de proposta de recurso](../.github/ISSUE_TEMPLATE/feature.yml).

- Campos globais: `kind` (`none`, `fade`, `pop`, `slide`), `open_ms`/`close_ms` separados, `open_easing`/`close_easing`, `pop_scale` (padrão `0.85`), `slide_direction` (`nearest` por padrão, ou `top`, `right`, `bottom`, `left`), `slide_offset_px` opcional (ausente significa trajeto até a borda) e `suppress_workspace_switch` (padrão proposto `true`).
- Padrões de compatibilidade: fade, smoothstep nas duas direções e durações herdadas de `fade_ms` (180 ms quando ausente). As novas durações explícitas prevalecem sobre o valor antigo por direção; zero conclui imediatamente. Os exemplos opcionais de pop/slide usam 220 ms para abrir e 150 ms para fechar. A refatoração preserva exatamente o fade das janelas elegíveis; novas exclusões e supressão em trocas de área são mudanças intencionais de elegibilidade.
- Regras usam metadados em cache do cliente: classe de recurso de `WM_CLASS`, `_NET_WM_WINDOW_TYPE` e nome (`_NET_WM_NAME`, com alternativa em `WM_NAME`). Reutilizar a associação cliente/moldura, validar tipos/tamanhos das propriedades e preservar os metadados após destruição. A proposta é comparação textual exata e sensível a maiúsculas, com todos os seletores informados satisfeitos; a primeira regra compatível substitui somente os campos globais especificados. Propriedades ausentes/malformadas não satisfazem seletores. Atualizar metadados para transições futuras sem reiniciar uma animação ativa apenas porque o título mudou.
- Excluir tooltips override-redirect e qualquer superfície realmente fora da composição; regras podem excluir outras janelas com `kind = "none"`. Essas exclusões obrigatórias prevalecem sobre regras que habilitam animações. Menus dropdown override-redirect continuam elegíveis para regras explícitas de slide curto. Não implementar suspensão da composição em tela cheia neste marco; preservar o contrato de exclusão quando esse recurso for introduzido.
- Rejeitar chaves/tipos/curvas desconhecidos, durações inválidas, escalas não finitas ou não positivas e deslocamentos fora do intervalo antes da conexão X11. As regras são carregadas uma vez junto com a configuração; recarga é trabalho separado.

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
3. **Adicionar configuração, metadados, regras e exclusões.** **Problema ou caso de uso:** `fade_ms` global não expressa políticas por janela. **Comportamento proposto:** ampliar `config.rs`, `atoms.rs` e metadados de clientes/superfícies com resolução TOML estrita, primeira regra compatível, opções distintas por direção e exclusões; manter os padrões legados. **Como verificar:** parsing/precedência cobrem dados válidos, ausentes, malformados e conflitantes; casos de moldura/cliente e cliente destruído selecionam a política correta. Expor cada tipo novo de animação somente quando sua tarefa de renderização estiver pronta.
4. **Introduzir transformações no XRender.** **Problema ou caso de uso:** pintura/desfoque/recortes usam geometria sem transformação. **Comportamento proposto:** adicionar amostragem afim centrada, transformações inversas de imagem, limites de destino, cobertura consistente de efeitos e estado reutilizável de transformação/filtro em `picture.rs` e `renderer/{paint,blur}.rs`. **Como verificar:** identidade reproduz os pixels atuais; amostras controladas de escala/deslocamento preservam centros, formatos, bordas, transparência, desfoque e recortes extremos sem crescimento de recursos por quadro.
5. **Implementar pop e curvas selecionáveis.** **Problema ou caso de uso:** o caminho genérico precisa de um efeito completo de escala/opacidade. **Comportamento proposto:** adicionar extremos 0,85→1 do pop, amostragem cúbica/back/linear, opacidade/escala limitadas e curvas configuráveis por direção. **Como verificar:** casos puros de extremos/ultrapassagem/interrupção e cenas reais de abertura, unmap, destroy e remapeamento passam com preservação do último conteúdo.
6. **Implementar slide por saída e deslocamentos de menu.** **Problema ou caso de uso:** bordas da raiz não representam cada monitor. **Comportamento proposto:** manter geometria negociada de monitores/CRTCs RandR, atualizar em mudanças de saída/CRTC, selecionar a saída da janela e adicionar direções automáticas/explícitas, deslocamentos dropdown e alternativa documentada. **Como verificar:** casos geométricos determinísticos e capturas cobrem resoluções diferentes, origens negativas, empates, janelas entre saídas, hotplug/resize durante animação e ausência de informações RandR.
7. **Suprimir animações provocadas por troca de área.** **Problema ou caso de uso:** a transição pode parecer várias aberturas/fechamentos independentes. **Comportamento proposto:** adicionar acompanhamento da área na raiz e classificação limitada do ciclo de vida em `atoms.rs`, `events.rs` e `compositor.rs`; concluir capturas afetadas e preservar transições posteriores reais. **Como verificar:** cenários de protocolo reordenados/em lotes e uma sessão real do Xmonad passam nos casos de troca, troca rápida, animação interrompida e abertura comum posterior; registrar comportamentos de gerenciadores não cobertos.
8. **Validar desempenho, limpeza e documentação.** **Problema ou caso de uso:** correção visual sozinha não comprova ociosidade nem estabilidade de recursos. **Comportamento proposto:** ampliar `tests/cases/`, os auxiliares XRes e o programa de verificação de desktop para transformações simultâneas, ciclos repetidos e cadência medida; atualizar README, arquitetura, exemplo TOML válido e os dois roteiros quando implementado. **Como verificar:** cada item de aceitação possui evidências do commit exato e ambiente declarado; recursos apenas propostos continuam marcados como planejados até passar nas verificações.

### Decisões e trabalho posterior

O responsável pelo projeto ainda precisa confirmar o esquema público `[animations]`/`[[animation_rules]]`, a primeira regra com comparação textual exata e os exemplos opcionais de 220/150 ms. O plano preserva o fade legado padrão de 180 ms e propõe habilitar supressão em trocas de área. Nenhuma data de versão ou novo critério da beta foi atribuído.

Animar movimento/redimensionamento de janelas existentes e transições de slide da área de trabalho inteira está fora do escopo e permanece como trabalho futuro do roteiro. Modelo de mola, sombras, cantos arredondados, suspensão da composição em tela cheia, redesenho parcial e recarga de configuração continuam separados.

Ainda não há datas de entrega. Abra uma issue para discutir uma mudança delimitada ou relatar uma falha observada; evite iniciar vários projetos de backend sobrepostos antes de alinhar os requisitos.
