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
| 1. Estabilidade das janelas | Em andamento | Ciclo de vida, menus, transições de tela cheia e propriedades inválidas com cobertura reproduzível, sem quedas nem janelas invisíveis ou imagens antigas. |
| 2. Monitores e recursos | Pendente | Mudanças de resolução, conexão/desconexão de monitores, recuperação da apresentação e consumo de recursos em redimensionamentos repetidos verificados. |
| 3. Desktops reais | Pendente | Sessões Xorg/XLibre com registro de gerenciadores e drivers testados, além de medições de CPU, memória e regularidade dos quadros. |
| 4. Distribuição da beta | Pendente | Pré-lançamento versionado com instruções de instalação e execução, limitações conhecidas, artefatos verificados e procedimento reproduzível para relatar falhas. |

### 1. Estabilidade das janelas

Exercitar sequências rápidas de map/unmap/destroy, fades interrompidos, janelas decoradas, menus override-redirect, entrada e saída de tela cheia, propriedades malformadas e destruição durante requisições do protocolo. Começar por regressões determinísticas de pixels no Xvfb; registrar o comportamento de gerenciadores reais na etapa 3.

**Aceitação:** cada defeito reproduzido possui uma regressão que falha sem a correção; os cenários cobertos preservam os pixels corretos e mantêm o compositor em execução; suíte completa, formatação, Clippy e build de release passam. Os cenários restantes continuam explicitamente abertos até serem exercitados. O acompanhamento entre cliente e moldura, a validação de propriedades e um primeiro conjunto de sequências rápidas estão cobertos abaixo. Fades interrompidos em servidor real, formatos extremos ou fora da tela e uma cobertura maior de corridas com destruição continuam abertos nesta etapa.

### 2. Monitores e recursos

Testar mudanças de resolução via RandR e hotplug físico, recuperação das falhas de apresentação previstas e redimensionamentos repetidos com contagem de recursos do servidor X. Registrar a configuração e a disposição dos monitores utilizadas.

**Aceitação:** a imagem se recupera após cada transição coberta, a apresentação continua e as operações repetidas não provocam crescimento indefinido de memória ou recursos do servidor.

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

A suíte completa agora contém 32 testes: cinco unitários, três de CLI e vinte e quatro de integração X11. A etapa 1 continua em andamento; este conjunto não encerra todos os seus cenários de aceitação.

| Ambiente | Cobertura verificada | Evidência / limites |
| --- | --- | --- |
| Xvfb 21.1.24 no CachyOS, 320×240×24, XRender e Present 1.2 | Associação entre cliente e moldura, propriedades malformadas e sequências rápidas; todos os 32 testes passam | Registro de 2026-10-02, `fade_ms = 0`, `blur_radius = 0` e vsync padrão nestes casos. Os testes criam as hierarquias diretamente; não validam um gerenciador de janelas real nem uma GPU. |
| Xorg com gerenciador de janelas real e drivers Intel/AMD/NVIDIA | Pendente | Exige registro de servidor, gerenciador, driver, configuração e commit. |
| XLibre com gerenciador de janelas real e drivers Intel/AMD/NVIDIA | Pendente | Exige os mesmos registros de ambiente; os resultados no Xvfb não comprovam suporte. |

Reproduza as verificações com a toolchain fixada pelo repositório e o Xvfb instalado. As [execuções de CI](https://github.com/hashdefault/compust/actions/workflows/ci.yml) registram resultados para cada commit exato; inclua a revisão exibida abaixo nos relatos locais.

```sh
git rev-parse HEAD
cargo test --locked --test x11 client_lifecycle
cargo test --locked --test x11 properties
cargo test --locked --test x11 stability
```

Defina `XVFB=/caminho/para/Xvfb` se o servidor estiver fora de `PATH`.

### Critérios ainda pendentes

Teste Xorg e XLibre com gerenciadores de janelas reais e drivers Intel, AMD e NVIDIA. Registre servidor, driver, configuração e commit em cada relato. Cubra mudanças de parentesco após a inicialização, sequências rápidas de map/unmap/destroy, janelas decoradas e override-redirect, menus, tela cheia, ferramentas de papel de parede e encerramento da sessão.

Acrescente cobertura de redimensionamento RandR e hotplug reais, recuperação de falhas de apresentação e contagem de recursos durante redimensionamentos repetidos. Verifique formatos grandes ou fora da tela e propriedades malformadas. Amplie a cobertura de corridas com destruição para além da descoberta do cliente e das leituras de opacidade, sem esconder outros erros do X.

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
