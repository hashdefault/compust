# Arquitetura

[English (US)](ARCHITECTURE.md) | [Português (Brasil)](ARCHITECTURE.pt-BR.md)

O Compust é um cliente X11 que possui a seleção de compositor de uma tela. O servidor X continua responsável pela entrada, pelas conexões dos aplicativos e pelo hardware de exibição. O gerenciador de janelas continua responsável por posicionamento, decorações, foco e áreas de trabalho.

## Da janela ao quadro

```text
Aplicativo desenha em uma janela redirecionada
                     |
           Damage / eventos de janela
                     |
       Cena: empilhamento e tempo de vida
                     |
   XRender: fundo -> desfoque -> janelas
                     |
       Cópia com Present ou XRender
                     |
     Overlay do Composite (sem região de entrada)
```

`session.rs` negocia as capacidades e adquire `_NET_WM_CM_Sn`. Um bloqueio breve do servidor torna a verificação do proprietário e a aquisição do redirecionamento atômicas em relação aos outros clientes. O bloqueio também é liberado em caso de erro. Um timestamp real do servidor é obtido antes de adquirir a seleção e anunciar `MANAGER`.

`surface.rs` captura cada janela visível de nível superior em um pixmap nomeado. Damage é associado ao pixmap, que permanece válido até o término da animação de fechamento, mesmo que a janela original desapareça. `picture.rs` controla as imagens XRender e os pixmaps próprios; o encerramento da última conexão também libera recursos e redirecionamentos após falhas na inicialização.

`scene.rs` mantém as superfícies na ordem de empilhamento do servidor. Superfícies de janelas destruídas permanecem abaixo da antiga vizinha superior até o fim do fade. Remapear uma janela substitui seu conteúdo somente após uma captura bem-sucedida, preservando a opacidade corrente para a reabertura. `events.rs` atualiza geometria, tempo de vida, formato e opacidade. O servidor dá à janela um novo pixmap a cada redimensionamento; por isso uma superfície é recapturada quando o tamanho informado em um `ConfigureNotify` ou o tamanho atual da janela difere do capturado. Comparar só o tamanho atual deixa passar uma janela redimensionada e restaurada antes de o evento ser tratado. `surface/client.rs` descobre o aplicativo por `WM_STATE` em até oito níveis de descendentes. Cada janela visitada recebe uma assinatura de eventos de propriedade e de janelas filhas antes da leitura de seu estado. A criação tardia ou remoção de `WM_STATE`, a criação de filhos, as mudanças de parentesco e a destruição atualizam a associação e a opacidade da moldura afetada. A propriedade de opacidade da moldura tem precedência quando existe.

Um descendente que desaparece durante a descoberta é ignorado somente no caso de `BadWindow`. Se o cliente selecionado desaparecer antes da leitura de opacidade, a moldura sobrevivente usa sua própria opacidade ou o padrão opaco; os eventos seguintes de ciclo de vida atualizam a associação. Perder um cliente não deve encerrar a superfície da moldura. Trata-se de suporte limitado às convenções dos gerenciadores de janelas, não de uma implementação completa de EWMH/ICCCM.

As leituras de propriedades validam a representação no protocolo antes de usar os dados dos aplicativos. `WM_STATE` deve ter tipo `WM_STATE`, formato 32 e exatamente dois valores. `_NET_WM_WINDOW_OPACITY` deve conter um único `CARDINAL` de 32 bits, sem dados excedentes na propriedade. Valores inválidos são tratados como ausentes, inclusive permitindo usar a opacidade válida do cliente quando a da moldura está malformada. Erros normais de requisições X continuam seguindo os tratamentos existentes.

## Renderização e agendamento

O renderizador desenha o papel de parede em um buffer reutilizável com a profundidade da raiz, compõe as janelas de baixo para cima e aplica uma máscara A8 para a opacidade efetiva. O alfa por pixel permanece na imagem de origem. Os retângulos de Shape recortam tanto a janela quanto o desfoque. Eles usam coordenadas locais do pixmap, incluindo a borda; a origem de recorte do XRender aplica a posição da janela. Isso evita saturar prematuramente a soma de coordenadas de 16 bits para formatos extremos fora da tela. O módulo de desfoque monta uma pirâmide em torno de cada janela translúcida: cada nível reduz o anterior pela metade com amostragem bilinear, e depois o nível mais grosso é ampliado de volta, nível por nível, dentro do formato da janela. O raio escolhe a profundidade, arredondada para 2, 4, 8 ou 16 pixels. A área da pirâmide recebe uma margem de duas vezes o passo mais grosso e se alinha a esse passo, para que pixels deixados por quadros anteriores não alcancem a janela e o resultado continue ancorado à tela quando as janelas se movem. Os buffers dos níveis só existem com o desfoque ativado. Drivers baseados em glamor aceleram essas transformações bilineares, mas processam na CPU os filtros de convolução usados por um desfoque de caixa anterior; o [guia de validação de desktops](DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03) compara os dois.

`animation.rs` usa `Instant` monotônico e interpolação smoothstep inteira. O fechamento e a reabertura partem da opacidade amostrada naquele momento, preservando a continuidade de animações interrompidas. O loop agenda um quadro final no ponto de chegada, inclusive quando a duração é zero.

`compositor.rs` processa lotes limitados de eventos para que uma tempestade de eventos não adie indefinidamente a pintura. Há redesenho somente após dano, mudanças relevantes ou durante animações, respeitando `max_fps` como teto. Quando ocioso, o socket X é aguardado por no máximo um segundo para observar os sinais de encerramento e de recarga. Damage ainda provoca redesenho da tela inteira; não é uma otimização por regiões.

`config.rs` lê o arquivo de `--config` ou, sem ele, o primeiro `compust/compust.toml` nos diretórios de configuração XDG. SIGUSR1 marca um indicador que o loop de eventos confere após cada lote de eventos. A recarga repete essa busca e, portanto, lê o que uma reinicialização leria; um arquivo ilegível ou inválido mantém a configuração em uso. A opacidade e o teto de quadros são lidos a cada quadro. Cada transição de fade usa a duração configurada no momento em que começa, então um `fade_ms` recarregado vale também para janelas abertas antes. O renderizador registra o raio de desfoque e a opção vsync com que foi criado, e uma recarga que altere qualquer um deles o substitui na pintura seguinte, depois que Present libera o buffer, como em qualquer quadro. Os tratadores de sinais são instalados antes da tomada da seleção do compositor: um cliente pode sinalizar o processo assim que vê a seleção, e SIGUSR1 o encerraria. SIGUSR1 segue o picom; SIGHUP é evitado porque um terminal ao ser fechado o envia.

Com Present, um único buffer é enviado no modo COPY e só volta a ser usado após as notificações de conclusão e liberação. Uma mudança de tela via RandR é a exceção: o renderizador é substituído sem esperar, porque uma reconfiguração de CRTC em hardware modesetting/amdgpu descartou os dois eventos de um envio pendente. Um tempo limite de um segundo cobre perdas por qualquer outra causa: o renderizador é substituído e o quadro é repintado uma vez. Se o envio seguinte também exceder o tempo limite, os buffers são substituídos de novo, mas a pintura espera por novo dano, para que um servidor que parou de informar não cause uma repintura por segundo. Um `ConfigureNotify` da raiz pode solicitar essa substituição, mas não cancelá-la. Os IDs de assinatura distinguem eventos antigos depois da recriação dos buffers, e assinaturas anteriores são liberadas. É uma base simples, não um agendador de baixa latência com vários buffers. Sem Present, XRender copia diretamente para a janela de composição.

Um `BadMatch` na requisição PresentPixmap pendente exata pode acionar a cópia via XRender após consultas de geometria confirmarem que o pixmap original e a saída ainda existem na mesma tela, com profundidades iguais e as dimensões esperadas do buffer. A requisição rejeitada não retém o buffer, então a espera por conclusão e liberação é encerrada. A sessão passa então a considerar Present indisponível, e a alternativa se mantém quando um redimensionamento da raiz ou uma recarga de configuração recria o renderizador. Um aviso registra a requisição rejeitada. Recursos inválidos e erros não relacionados não fazem parte dessa recuperação.

## Mapa do código

| Área | Arquivos |
| --- | --- |
| Inicialização e configuração | `main.rs`, `config.rs` |
| Propriedade e capacidades X11 | `session.rs`, `atoms.rs`, `capabilities.rs` |
| Recursos capturados e tempo de vida | `picture.rs`, `surface.rs` |
| Associação entre cliente e moldura | `surface/client.rs` |
| Cena e eventos | `scene.rs`, `events.rs`, `compositor.rs` |
| Efeitos e apresentação | `animation.rs`, `renderer.rs`, `renderer/{paint,blur,present,wallpaper}.rs` |
| Verificação com servidor real | `tests/x11.rs`, `tests/cases/`, `tests/support/` |

Os caminhos de código são relativos a `src/`, exceto os de teste. O crate usa `unsafe_code = "forbid"`; essa restrição vale para o próprio crate, não para os detalhes internos das dependências. Rust evita várias classes de erro de memória, mas corridas do protocolo, erros de renderização e vazamentos de recursos ainda exigem testes.

## Verificação da captura

Os testes de corridas durante a captura encaminham apenas a conexão do compositor por `tests/support/proxy.rs`. O proxy delimita requisições X11 na ordem de bytes nativa usando x11rb, encaminha respostas e eventos sem alteração e aguarda sua thread terminar no encerramento. `tests/support/capture.rs` consulta os opcodes das extensões no Xvfb e conclui uma destruição de janela com confirmação imediatamente antes de encaminhar a requisição escolhida. O cliente de teste mantém uma conexão direta com o servidor para conferir pixels e recursos. Esse transporte pertence somente ao binário de testes; a execução normal do compositor não possui pontos de interceptação.

O mesmo transporte atende aos testes de apresentação substituindo um campo da requisição antes do envio; o Xvfb produz o erro real. Substituir a fence SYNC de espera por uma que nunca é acionada retém, em vez disso, os eventos de conclusão e liberação de um envio. `tests/support/monitors.rs` usa requisições RandR com confirmação no servidor isolado daquele teste. `tests/support/resources.rs` consulta XRes usando a janela proprietária da seleção do compositor como identificador do cliente. Ele compara contagens de recursos e os bytes totais reportados para os XIDs dos pixmaps próprios em estados renderizados equivalentes após aquecimento. `QueryResourceBytes` do XRes 1.2 fornece esses tamanhos sem dividi-los pela contagem de referências; a atribuição de `QueryClientPixmapBytes` pode variar enquanto Present mantém uma referência. O teste exige tamanhos para todos os pixmaps próprios e exclui referências cruzadas para evitar contagem duplicada. XRes é habilitado apenas para desenvolvimento; o compositor de produção não exige a extensão. As capturas dos testes usam as dimensões atuais do overlay.

## Como estender

Um backend de GPU deve preservar os contratos da cena e dos tempos de vida ao substituir a renderização. Evite criar uma abstração genérica antes de entender os requisitos reais de importação, sincronização e apresentação. As consultas de versão de DRI3 e Sync servem apenas ao diagnóstico neste momento.

A próxima otimização deve partir de medições: regiões danificadas precisam incluir as dependências do desfoque, e o descarte de áreas ocultas precisa considerar transparência e formato. Meça também o servidor X, pois o XRender delega trabalho a ele. O [roteiro](ROADMAP.pt-BR.md) define critérios de aceitação para essas mudanças.
