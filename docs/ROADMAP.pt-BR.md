# Roteiro de desenvolvimento

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

O objetivo é tornar este compositor mínimo em Rust uma opção prática para usuários de Xorg e XLibre. “Melhor que o picom” precisa significar melhorias observáveis em confiabilidade, latência, consumo de recursos ou manutenção. Reescrever um recurso em Rust, por si só, não comprova melhor desempenho.

## Base 0.1: implementada

O repositório contém um compositor executável com composição XRender, transições de opacidade, transparência alfa, desfoque por convolução, recorte por formato, acompanhamento de atualizações, empilhamento, tratamento de redimensionamento de janelas, propriedades de papel de parede e apresentação opcional por cópia com Present. A suíte verifica o comportamento de um servidor real no Xvfb. A documentação e as orientações de contribuição estão em inglês e pt-BR.

Esse marco cria uma base para experimentação. Ele não comprova compatibilidade completa com ambientes gráficos nem desempenho em hardware real.

## Primeira beta: quatro etapas

A primeira beta usará o backend XRender atual e declarará suporte somente aos ambientes com evidências registradas de teste. Ela será voltada a testes controlados da comunidade. A versão 0.1.0 continua sendo um protótipo experimental; definir estes critérios não a transforma em uma versão beta.

| Etapa | Estado | Resultado necessário |
| --- | --- | --- |
| 1. Estabilidade das janelas | Concluída no Xvfb | Ciclo de vida, menus, transições de tela cheia e propriedades inválidas com cobertura reproduzível, sem quedas nem janelas invisíveis ou imagens antigas. |
| 2. Monitores e recursos | Verificações automatizadas concluídas; hotplug físico pendente | Mudanças de resolução, conexão/desconexão de monitores, recuperação da apresentação e consumo de recursos em redimensionamentos repetidos verificados. |
| 3. Desktops reais | Pendente | Sessões Xorg/XLibre com registro de gerenciadores e drivers testados, além de medições de CPU, memória e regularidade dos quadros. |
| 4. Distribuição da beta | Pendente | Pré-lançamento versionado com instruções de instalação e execução, limitações conhecidas, artefatos verificados e procedimento reproduzível para relatar falhas. |

### 1. Estabilidade das janelas

Exercitar sequências rápidas de map/unmap/destroy, fades interrompidos, janelas decoradas, menus override-redirect, entrada e saída de tela cheia, propriedades malformadas e destruição durante requisições do protocolo. Começar por regressões determinísticas de pixels no Xvfb; registrar o comportamento de gerenciadores reais na etapa 3.

**Aceitação:** cada defeito reproduzido possui uma regressão que falha sem a correção; os cenários cobertos preservam os pixels corretos e mantêm o compositor em execução; suíte completa, formatação, Clippy e build de release passam. A cobertura abaixo inclui clientes e molduras, propriedades, sequências rápidas, fades interrompidos, formatos extremos ou fora da tela e destruição antes e entre requisições de captura. A validação automatizada no Xvfb está concluída; a validação de gerenciadores e drivers reais continua na etapa 3.

### 2. Monitores e recursos

Testar mudanças de resolução via RandR e hotplug físico, recuperação das falhas de apresentação previstas e redimensionamentos repetidos com contagem de recursos do servidor X. Registrar a configuração e a disposição dos monitores utilizadas.

**Aceitação:** a imagem se recupera após cada transição coberta, a apresentação continua e as operações repetidas não provocam crescimento indefinido de memória ou recursos do servidor.

Os cenários com Xvfb descritos abaixo passam. Hotplug físico e configurações com vários monitores continuam em aberto; desativar um CRTC virtual não comprova esses comportamentos.

### 3. Desktops reais

Executar cenários documentados com gerenciadores de janelas reais em Xorg e XLibre. Registrar servidor, gerenciador, GPU/driver, configuração e commit exato. Medir CPU ociosa e em atividade, memória e regularidade dos quadros; corrigir falhas nos ambientes propostos para suporte na beta.

**Aceitação:** publicar uma matriz de compatibilidade com evidências para cada ambiente anunciado, uma referência reproduzível de medições e as limitações restantes. Combinações de servidor e driver ainda não testadas permanecem sem validação.

### 4. Distribuição da beta

Publicar uma prévia beta versionada com instruções de compilação ou instalação do binário, exemplos de configuração, orientações de início e encerramento, checksums dos artefatos distribuídos, limitações conhecidas e um modelo de relato com diagnóstico e passos de reprodução.

**Aceitação:** uma pessoa consegue instalar e executar a versão exata, retornar ao compositor anterior e relatar uma falha seguindo as instruções fornecidas. O CI passa para o commit da versão, e as três etapas anteriores estão aprovadas dentro do escopo de suporte declarado.

Ainda não há data para a beta. São quatro critérios de liberação, não uma quantidade fixa de commits. A expansão de backend de GPU e os efeitos avançados podem vir depois da primeira beta.

## Progresso e verificação

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

Três testes em [presentation.rs](../tests/cases/presentation.rs) cobrem envios rejeitados. Um proxy de teste substitui o pixmap em uma requisição Present por outro incompatível, produzindo um `BadMatch` do servidor real. Antes, o compositor encerrava. Agora ele identifica o envio exato, verifica se o buffer original e a saída ainda existem com tela e profundidade compatíveis e continua com XRender pelo restante da sessão, inclusive após redimensionar a raiz. Erros de pixmap ou janela inválidos continuam encerrando o compositor. Ausência de eventos de conclusão e outros erros de apresentação estão fora dessa política de recuperação.

A suíte completa agora tem 59 testes aprovados: seis unitários, três de CLI e cinquenta de integração X11. Formatação, Clippy estrito, build de release e verificações da documentação passam. A etapa 2 continua aberta para hotplug físico, configurações com vários monitores e medições em hardware. A configuração automatizada usa uma saída virtual de 320×240, temporariamente 240×180, com `fade_ms = 0`, `blur_radius = 0` e cada modo de vsync.

| Ambiente | Cobertura verificada | Evidência / limites |
| --- | --- | --- |
| Xvfb 21.1.24 no CachyOS, 320×240×24, XRender e Present 1.2 | Clientes/molduras, propriedades, sequências rápidas, fades interrompidos, formatos, destruição com eventos pendentes e onze pontos da captura; todos os 47 testes passam | Registro de 2026-10-02. Corridas de captura usam `fade_ms = 0`, `blur_radius = 0` e vsync padrão. Os casos anteriores de fades/formatos também usam `fade_ms = 1000`, `blur_radius = 4` ou `vsync = false`, conforme descrito acima. Hierarquias criadas diretamente, sem validação de gerenciador real ou GPU. |
| Mesmo Xvfb, uma saída virtual, RandR e XRes | Redimensionamento da raiz, desativação/restauração do CRTC, envio Present rejeitado e contagem repetida de recursos; todos os 59 testes passam | Registro de 2026-10-02 (horário local). Contagens e bytes do servidor são conferidos em estados renderizados equivalentes; hotplug físico e vários monitores ainda não foram verificados. |
| Xorg com gerenciador de janelas real e drivers Intel/AMD/NVIDIA | Pendente | Exige registro de servidor, gerenciador, driver, configuração e commit. |
| XLibre com gerenciador de janelas real e drivers Intel/AMD/NVIDIA | Pendente | Exige os mesmos registros de ambiente; os resultados no Xvfb não comprovam suporte. |

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

Teste Xorg e XLibre com gerenciadores de janelas reais e drivers Intel, AMD e NVIDIA. Registre servidor, driver, configuração e commit em cada relato. Cubra mudanças de parentesco após a inicialização, sequências rápidas de map/unmap/destroy, janelas decoradas e override-redirect, menus, tela cheia, ferramentas de papel de parede e encerramento da sessão.

Exercite hotplug físico e configurações com vários monitores em hardware real, além de medir memória e apresentação em execuções mais longas. As transições RandR virtuais, a recuperação restrita de envios Present rejeitados e a contagem repetida via XRes acima estão concluídas. Preserve a cobertura de destruição entre requisições de captura, liberação de recursos, formatos grandes ou fora da tela e propriedades malformadas registrada acima.

**Aceitação:** reproduções documentadas viram testes quando viável; o uso normal não causa quedas nem deixa janelas invisíveis ou imagens antigas; mudanças repetidas de ciclo de vida não fazem os recursos do servidor crescerem indefinidamente. Mantenha uma matriz de compatibilidade com evidências.

## Depois: medir e reduzir o trabalho de renderização

Colete referências em builds de release para CPU ociosa, CPU do aplicativo e do servidor X, memória, regularidade dos quadros e latência entre entrada e exibição. Compare cenas e efeitos equivalentes com uma versão/backend registrados do picom. Inclua alta resolução e monitores com taxas diferentes.

Implemente regiões de dano, expansão das regiões de desfoque, descarte de áreas ocultas e cache somente quando a medição justificar a complexidade. Avalie múltiplos buffers de apresentação com controle explícito de propriedade e conclusão.

**Aceitação:** um benchmark reproduzível demonstra a melhora, os testes de pixels continuam corretos nas bordas das regiões e o trabalho ocioso não aumenta. Publique o benefício e as cargas em que ele deixa de existir.

## Backend e expansão do protocolo

Avalie um backend EGL/OpenGL ou Vulkan depois de especificar importação e sincronização. A integração moderna com X11 pode envolver DMA-BUF por DRI3, negociação de modificadores, agendamento por Present e sincronização explícita. Cada parte exige implementação real e testes de driver; consultas de versão não contam como suporte.

Gerenciamento de cores, HDR, VRR, extensões específicas do XLibre e agendamento por saída exigem investigação de protocolo e hardware antes de qualquer promessa. Wayland nativo está fora deste roteiro.

**Aceitação:** o backend importa conteúdo real sem cópia rotineira para a CPU, respeita o tempo de vida dos buffers, recupera-se das falhas previstas e apresenta cobertura de drivers e medições documentadas.

## Uso cotidiano

Adicione regras por janela com tipos definidos, descoberta e recarga de configuração, diagnóstico mais claro e empacotamento para distribuições. Amplie as animações para movimento e escala somente depois de resolver a interação com coordenadas de entrada e geometria do gerenciador. Considere sombras e cantos arredondados com tratamento correto de formato e dano.

**Aceitação:** o comportamento é configurável, documentado nos dois idiomas e testável, sem anunciar implicitamente compatibilidade com a configuração ou a linguagem de animação do picom.

Ainda não há datas de entrega. Abra uma issue para discutir uma mudança delimitada ou relatar uma falha observada; evite iniciar vários projetos de backend sobrepostos antes de alinhar os requisitos.
