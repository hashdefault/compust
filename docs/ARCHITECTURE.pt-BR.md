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

`scene.rs` mantém as superfícies na ordem de empilhamento do servidor. Ele espelha os filhos da raiz a partir dos eventos de criação, destruição, mudança de parentesco, configuração e circulação, de modo que reempilhar não custa uma ida e volta de `QueryTree`. A resposta de uma consulta já reflete todo evento que o servidor gerou antes de tratá-la; por isso, eventos com número de sequência anterior não alteram o espelho, e um evento que cite uma janela ausente do espelho provoca nova consulta. Superfícies de janelas destruídas permanecem abaixo da antiga vizinha superior até o fim do fade. Remapear uma janela substitui seu conteúdo somente após uma captura bem-sucedida, preservando a opacidade corrente para a reabertura. `events.rs` atualiza geometria, tempo de vida, formato e opacidade. O servidor dá à janela um novo pixmap a cada redimensionamento e informa cada redimensionamento; por isso uma superfície é recapturada quando o tamanho informado em um `ConfigureNotify` difere do capturado. Isso inclui uma janela redimensionada e restaurada antes de os eventos serem tratados, cujo primeiro evento traz o tamanho intermediário; comparar só o tamanho atual a deixaria passar. Um movimento apenas atualiza a posição a partir do evento, sem ida e volta. `surface/client.rs` descobre o aplicativo por `WM_STATE` em até oito níveis de descendentes. Cada janela visitada recebe uma assinatura de eventos de propriedade e de janelas filhas antes da leitura de seu estado. A criação tardia ou remoção de `WM_STATE`, a criação de filhos, as mudanças de parentesco e a destruição atualizam a associação e a opacidade da moldura afetada. A propriedade de opacidade da moldura tem precedência quando existe.

Um descendente que desaparece durante a descoberta é ignorado somente no caso de `BadWindow`. Se o cliente selecionado desaparecer antes da leitura de opacidade, a moldura sobrevivente usa sua própria opacidade ou o padrão opaco; os eventos seguintes de ciclo de vida atualizam a associação. Perder um cliente não deve encerrar a superfície da moldura. Trata-se de suporte limitado às convenções dos gerenciadores de janelas, não de uma implementação completa de EWMH/ICCCM.

As leituras de propriedades validam a representação no protocolo antes de usar os dados dos aplicativos. `WM_STATE` deve ter tipo `WM_STATE`, formato 32 e exatamente dois valores. `_NET_WM_WINDOW_OPACITY` deve conter um único `CARDINAL` de 32 bits, sem dados excedentes na propriedade. Valores inválidos são tratados como ausentes, inclusive permitindo usar a opacidade válida do cliente quando a da moldura está malformada. Erros normais de requisições X continuam seguindo os tratamentos existentes.

## Renderização e agendamento

O renderizador desenha o papel de parede em um buffer reutilizável com a profundidade da raiz, compõe as janelas de baixo para cima e aplica uma máscara A8 para a opacidade efetiva. O alfa por pixel permanece na imagem de origem. Os retângulos de Shape recortam tanto a janela quanto o desfoque. Eles são levados a coordenadas da raiz com aritmética de 32 bits e limitados à janela e à área a repintar antes de chegar ao servidor, então formatos extremos fora da tela não estouram coordenadas de 16 bits. O módulo de desfoque monta uma pirâmide em torno de cada janela translúcida: cada nível reduz o anterior pela metade com amostragem bilinear, e depois o nível mais grosso é ampliado de volta, nível por nível, em um buffer de fundo mostrado dentro do formato da janela. O raio escolhe a profundidade, arredondada para 2, 4, 8 ou 16 pixels. A área da pirâmide recebe uma margem de duas vezes o passo mais grosso e se alinha a esse passo, para que pixels deixados por quadros anteriores não alcancem a janela e o resultado continue ancorado à tela quando as janelas se movem. Os buffers dos níveis só existem com o desfoque ativado. Drivers baseados em glamor aceleram essas transformações bilineares, mas processam na CPU os filtros de convolução usados por um desfoque de caixa anterior; o [guia de validação de desktops](DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03) compara os dois.

`animation.rs` usa `Instant` monotônico e interpolação smoothstep inteira. O fechamento e a reabertura partem da opacidade amostrada naquele momento, preservando a continuidade de animações interrompidas. O loop agenda um quadro final no ponto de chegada, inclusive quando a duração é zero.

Cada quadro repinta apenas uma área da raiz. O Damage de cada janela informa o retângulo envolvente das mudanças desde o aviso anterior, o que as localiza sem buscar a região. `renderer/damage.rs` guarda o que o último quadro mostrou de cada superfície: limites, formato, imagem capturada, opacidade efetiva e posição no empilhamento. Cada mudança registra também a camada mais baixa da pilha que ela altera; o fundo é a camada 0. O conteúdo de uma superfície muda a partir da própria camada, assim como os limites antigos e novos de uma superfície que se moveu, foi redimensionada ou recapturada, mudou de formato ou opacidade, surgiu ou saiu. Uma reordenação muda a cena abaixo de cada superfície que trocou de lugar, nos limites das duas, mas a saída só onde elas se sobrepõem. Uma exposição do overlay muda apenas a saída; um renderizador novo ou uma troca de papel de parede partem do fundo, e uma rejeição do Present repinta tudo. `region.rs` guarda a área a repintar como no máximo 16 retângulos, unindo o par cujo envoltório acrescenta menos, então ela pode cobrir mais do que mudou, nunca menos. A pintura é recortada pela área, e um quadro de área vazia nem chega a ser pintado.

Cada superfície desfocada guarda seu fundo desfocado em um buffer que cobre seus limites na tela. O desfoque lê em volta de cada pixel que escreve, então uma mudança abaixo de uma superfície em qualquer ponto da sua área de alcance a desfoca de novo, o que repinta toda a área de alcance e muda a superfície para as que estão acima. Uma superfície repintada onde o fundo guardado não corresponde mais aos seus limites também é desfocada de novo. Qualquer outra repintura, como a mudança do próprio conteúdo da superfície, o fade da sua opacidade ou a mudança de uma janela acima dela, copia o fundo guardado e não custa desfoque. Os buffers são arredondados para 64 pixels e só são substituídos quando pequenos demais ou mais de quatro vezes grandes demais; um buffer sai quando sua superfície deixa de ser desfocada.

`compositor.rs` processa lotes limitados de eventos para que uma tempestade de eventos não adie indefinidamente a pintura. Há redesenho somente após dano, mudanças relevantes ou durante animações, respeitando `max_fps` como teto. Um evento que não altera nenhuma superfície exibida nem a ordem delas não provoca redesenho: com um único buffer do Present, esse quadro atrasaria o seguinte em um vblank. Quando ocioso, o socket X é aguardado por no máximo um segundo para observar os sinais de encerramento e de recarga.

`config.rs` lê o arquivo de `--config` ou, sem ele, o primeiro `compust/compust.toml` nos diretórios de configuração XDG. SIGUSR1 marca um indicador que o loop de eventos confere após cada lote de eventos. A recarga repete essa busca e, portanto, lê o que uma reinicialização leria; um arquivo ilegível ou inválido mantém a configuração em uso. A opacidade e o teto de quadros são lidos a cada quadro. Cada transição de fade usa a duração configurada no momento em que começa, então um `fade_ms` recarregado vale também para janelas abertas antes. O renderizador registra o raio de desfoque e a opção vsync com que foi criado, e uma recarga que altere qualquer um deles o substitui na pintura seguinte, depois que Present libera o buffer, como em qualquer quadro. Os tratadores de sinais são instalados antes da tomada da seleção do compositor: um cliente pode sinalizar o processo assim que vê a seleção, e SIGUSR1 o encerraria. SIGUSR1 segue o picom; SIGHUP é evitado porque um terminal ao ser fechado o envia.

Com Present, um único buffer é enviado no modo COPY com a área repintada como região de atualização, então o servidor copia apenas essa área; o buffer mantém o restante do quadro anterior, que ainda corresponde à tela. Ele só volta a ser usado após as notificações de conclusão e liberação. Uma mudança de tela via RandR é a exceção: o renderizador é substituído sem esperar, porque uma reconfiguração de CRTC em hardware modesetting/amdgpu descartou os dois eventos de um envio pendente. Um tempo limite de um segundo cobre perdas por qualquer outra causa: o renderizador é substituído e o quadro é repintado uma vez. Se o envio seguinte também exceder o tempo limite, os buffers são substituídos de novo, mas a pintura espera por novo dano, para que um servidor que parou de informar não cause uma repintura por segundo. Um `ConfigureNotify` da raiz pode solicitar essa substituição, mas não cancelá-la. Os IDs de assinatura distinguem eventos antigos depois da recriação dos buffers, e assinaturas anteriores são liberadas. É uma base simples, não um agendador de baixa latência com vários buffers. Sem Present, XRender copia a área diretamente para a janela de composição.

Um `BadMatch` na requisição PresentPixmap pendente exata pode acionar a cópia via XRender após consultas de geometria confirmarem que o pixmap original e a saída ainda existem na mesma tela, com profundidades iguais e as dimensões esperadas do buffer. A requisição rejeitada não retém o buffer, então a espera por conclusão e liberação é encerrada. A sessão passa então a considerar Present indisponível, e a alternativa se mantém quando um redimensionamento da raiz ou uma recarga de configuração recria o renderizador. Um aviso registra a requisição rejeitada. Recursos inválidos e erros não relacionados não fazem parte dessa recuperação.

## Mapa do código

| Área | Arquivos |
| --- | --- |
| Inicialização e configuração | `main.rs`, `config.rs` |
| Propriedade e capacidades X11 | `session.rs`, `atoms.rs`, `capabilities.rs` |
| Recursos capturados e tempo de vida | `picture.rs`, `surface.rs` |
| Associação entre cliente e moldura | `surface/client.rs` |
| Cena e eventos | `scene.rs`, `events.rs`, `compositor.rs` |
| Efeitos e apresentação | `animation.rs`, `region.rs`, `renderer.rs`, `renderer/{paint,damage,blur,present,wallpaper}.rs` |
| Verificação com servidor real | `tests/x11.rs`, `tests/cases/`, `tests/support/` |

Os caminhos de código são relativos a `src/`, exceto os de teste. O crate usa `unsafe_code = "forbid"`; essa restrição vale para o próprio crate, não para os detalhes internos das dependências. Rust evita várias classes de erro de memória, mas corridas do protocolo, erros de renderização e vazamentos de recursos ainda exigem testes.

## Verificação da captura

Os testes de corridas durante a captura encaminham apenas a conexão do compositor por `tests/support/proxy.rs`. O proxy delimita requisições X11 na ordem de bytes nativa usando x11rb, encaminha respostas e eventos sem alteração e aguarda sua thread terminar no encerramento. `tests/support/capture.rs` consulta os opcodes das extensões no Xvfb e conclui uma destruição de janela com confirmação imediatamente antes de encaminhar a requisição escolhida. O cliente de teste mantém uma conexão direta com o servidor para conferir pixels e recursos. Esse transporte pertence somente ao binário de testes; a execução normal do compositor não possui pontos de interceptação.

O mesmo transporte atende aos testes de apresentação substituindo um campo da requisição antes do envio; o Xvfb produz o erro real. Ele também registra a região de atualização de cada requisição PresentPixmap, para que os testes de regiões confiram quanto da tela cada quadro substituiu. Substituir a fence SYNC de espera por uma que nunca é acionada retém, em vez disso, os eventos de conclusão e liberação de um envio. `tests/support/monitors.rs` usa requisições RandR com confirmação no servidor isolado daquele teste. `tests/support/resources.rs` consulta XRes usando a janela proprietária da seleção do compositor como identificador do cliente. Ele compara contagens de recursos e os bytes totais reportados para os XIDs dos pixmaps próprios em estados renderizados equivalentes após aquecimento. `QueryResourceBytes` do XRes 1.2 fornece esses tamanhos sem dividi-los pela contagem de referências; a atribuição de `QueryClientPixmapBytes` pode variar enquanto Present mantém uma referência. O teste exige tamanhos para todos os pixmaps próprios e exclui referências cruzadas para evitar contagem duplicada. XRes é habilitado apenas para desenvolvimento; o compositor de produção não exige a extensão. As capturas dos testes usam as dimensões atuais do overlay.

## Como estender

Um backend de GPU deve preservar os contratos da cena e dos tempos de vida ao substituir a renderização. Evite criar uma abstração genérica antes de entender os requisitos reais de importação, sincronização e apresentação. As consultas de versão de DRI3 e Sync servem apenas ao diagnóstico neste momento.

A próxima otimização deve partir de medições: o descarte de áreas ocultas precisa considerar transparência, formato e a cena que um desfoque lê abaixo delas. Meça também o servidor X, pois o XRender delega trabalho a ele. O [roteiro](ROADMAP.pt-BR.md) define critérios de aceitação para essas mudanças.
