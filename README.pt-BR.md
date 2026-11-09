<div align="center">

<img src="assets/wordmark.png" width="440" alt="CPU ZeeZ">

**Reimplementação clean room, open source e multiplataforma das funcionalidades do CPU-Z, com monitor de hardware, escrita em Rust.**

Processador, caches, placa-mãe, memória e SPD, gráficos, sensores, discos, rede e bateria, benchmarks e relatórios TXT/HTML/JSON completos, no **Windows, Linux (Debian, Ubuntu, Arch, …) e macOS (Intel e Apple Silicon)**.

[![CI](https://github.com/joaoregis/cpuzeez/actions/workflows/ci.yml/badge.svg)](https://github.com/joaoregis/cpuzeez/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/joaoregis/cpuzeez)](https://github.com/joaoregis/cpuzeez/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#licença)

[English](README.md) · [Download](#instalação) · [Screenshots](#screenshots) · [Changelog](CHANGELOG.md) · [Cobertura](#cobertura-de-funcionalidades) · [Processo clean room](#processo-clean-room)

</div>

---

> O CPU ZeeZ (`cpuzeez`) **não tem vínculo com a CPUID**. CPU-Z é marca registrada da CPUID. Nenhum código, tabela ou texto do CPU-Z foi usado. Veja [Processo clean room](#processo-clean-room).

## Conteúdo

- [Destaques](#destaques)
- [Screenshots](#screenshots)
- [Instalação](#instalação)
- [Atualizações](#atualizações)
- [Acesso privilegiado ao hardware](#acesso-privilegiado-ao-hardware)
- [Qualidade, acessibilidade e segurança](#qualidade-acessibilidade-e-segurança)
- [Orçamento de desempenho](#orçamento-de-desempenho)
- [Uso](#uso)
- [Cobertura de funcionalidades](#cobertura-de-funcionalidades)
- [Precisão: lado a lado com o CPU-Z](#precisão-lado-a-lado-com-o-cpu-z)
- [Processo clean room](#processo-clean-room)
- [Compilando](#compilando)
- [Arquitetura](#arquitetura)
- [Contribuindo](#contribuindo)
- [Licença](#licença)

## Destaques

- **Interface moderna.**
  - Navegação em barra lateral e uma **Visão geral** ao vivo: um resumo do computador em uma linha, indicadores de saúde, blocos que você escolhe (com predefinições), ações rápidas e alertas locais opcionais.
  - **Saúde da montagem:** erros comuns de configuração e como corrigir (um módulo de memória só, XMP/EXPO desligado, placa de vídeo com menos linhas PCIe, Resizable BAR desligado, plano de economia de energia, monitor abaixo da taxa de atualização ou ligado na placa-mãe).
  - Uma **paleta de comandos** (`Ctrl+K`) que encontra qualquer campo, página ou ação; níveis de detalhe **Básico / Avançado / Especialista**; copiar ou **fixar** qualquer linha na Visão geral; unidades à sua escolha (°C/°F, MHz/GHz, MB/MiB).
  - Páginas em cards, temas claro e escuro, com a fonte de interface do próprio SO.
  - Disponível em English, Português (Brasil) e Español.
- **Tudo o que o CPU-Z mostra.**
  - CPU e topologia (CCD/CCX, núcleos P/E/LP).
  - Caches, placa-mãe, chipset e BIOS.
  - Timings de memória: AMD pela UMC, Intel pelo MCHBAR, de Skylake a Arrow Lake.
  - SPD: de DDR a DDR5 e LPDDR4/5, com XMP, EXPO e EPP.
  - Gráficos, PCI, DMI, timers e dumps de registradores.
  - As mesmas 23 seções do `-txt` do CPU-Z, na mesma ordem.
- **Monitor de hardware.**
  - Todos os sensores com mínimo, máximo e média, gráficos ao vivo e log em CSV.
  - **Detecção de limitação** (throttling) térmica, de potência e de corrente.
  - **Ícones na bandeja**, um por métrica que você escolher, como número, barra, minigráfico ou anel.
  - **Mini monitores:** quantas janelinhas sempre no topo você quiser, cada uma com seu layout (lista, barra, grade, um número grande ou o seu), que podem ser travadas (o clique atravessa) e somem enquanto um jogo em tela cheia roda.
  - Fontes de sensores:
    - **CPU:** temperaturas por núcleo, trilhos e PPT/TDC/EDC do SMU da AMD; RAPL Intel e AMD; clock efetivo por núcleo.
    - **GPU:** temperatura, hot spot, junção da memória, ventoinhas, potência, carga.
    - **Placa-mãe:** voltagens, temperaturas e ventoinhas do Super I/O, com mapas para mais de 150 modelos de placa.
    - **Chipset, DIMMs e discos.**
- **ZeeZ Live: overlay, FPS e captura, sem tocar no jogo.**
  - **Overlay:** uma janela própria, transparente, sempre no topo e que deixa os cliques passarem. Nada é injetado no jogo. Cinco estilos, nove posições mais um deslocamento (ou arraste), no monitor em que o jogo está, com contorno real no texto. **Etiquetas de redução de clock** dizem o que está reduzindo (CPU ou GPU, por temperatura ou potência).
  - **FPS:** PresentMon no Windows (incluso; exige administrador), MangoHud no Linux, Metal HUD no macOS (experimental). FPS, tempo de quadro, 1% e 0,1% lows, GPU Busy.
  - **Capture** uma sessão na mão, com timer, por atalho ou por jogo; ela é salva como `frames.csv` (colunas do PresentMon, então o CapFrameX abre), `hw.csv` e `session.json`. **Relatórios** com gráficos, comparação entre sessões e exportação em HTML.
  - **Atalhos globais** (no Wayland pelo portal de atalhos do desktop), perfis por jogo e um **modo seguro para anti-cheat** que para toda leitura via driver enquanto um jogo escolhido roda.
- **O que limita o seu FPS, medido.** O detector de gargalo diz *GPU no limite*, *CPU no limite (núcleo 4)*, *limitado de propósito* (V-Sync, um limite de FPS), *redução térmica*, *memória de vídeo cheia*, *paginação*, *disco* ou *outro programa*, com os números por trás, ao vivo e numa sessão inteira. Nunca ranqueia peças e nunca chuta.
- **Um Gerenciador de Tarefas em *Sistema*.** Processos (agrupados, com rede, energia, GPU, linha de comando, integridade e mais), Desempenho, **Histórico de apps** (opcional, guardado localmente), Inicialização com a linha do tempo do boot, Usuários e Serviços (recuperação, gatilhos, contas). Encerrar ou alterar algo pergunta antes e avisa quando o alvo é crítico para o sistema; o que exige direitos pede por eles só para aquela ação.
- **Benchmarks** (cargas próprias; 1000 pontos = o Ryzen 9 7900X / RTX 4060 de referência):
  - **Suíte de CPU:** compressão, n-corpos, matrizes com SIMD, AES com e sem as instruções AES, SHA-256, um ray tracer e ordenação, com uma e com várias threads; escala por threads, núcleos P × E e uma rodada sustentada que mostra a limitação.
  - **Memória:** latência e banda por nível de cache, STREAM triad, GUPS, o custo das falhas de TLB, latência por CCD.
  - **GPU** (DX12/Vulkan/Metal): FP32/FP16, banda da VRAM, o link PCIe nos dois sentidos, uma cena rasterizada.
  - **Armazenamento:** sequencial e aleatório 4K, sem cache, mais uma gravação sustentada que descobre o tamanho do cache do SSD.
  - **Mapa de calor da latência entre núcleos** e um **teste de estabilidade** (CPU leve/pesada/mista, teste de memória, estresse da GPU, ou tudo junto) que para sozinho nos limites de temperatura.
  - Cada resultado registra as condições em que rodou, pode ser salvo num **histórico** local e comparado subteste por subteste.
- **Dispositivos.**
  - **Discos:** **SMART** completo, autotestes SMART, varredura de superfície só de leitura, dados gravados frente à resistência nominal, alinhamento de partições, TRIM, criptografia e os dados de Identify decodificados.
  - **Monitores** (taxa de atualização, HDR, faixa de atualização), **USB** em árvore com a velocidade negociada, **Thunderbolt/USB4**, dispositivos de **áudio** com o formato, nível de bateria de dispositivos **Bluetooth**.
  - **Saúde da bateria** (desgaste, ciclos) e **adaptadores de rede** (link, IPs, DNS, driver, Wi-Fi).
- **Ferramentas.**
  - **Comparar relatórios**: o que mudou depois de uma troca de hardware ou de BIOS.
  - Clocks ao vivo de cada núcleo e da GPU, timers e exportação TXT/HTML/JSON.
- **A linha de comando do CPU-Z:**
  - As opções do CPU-Z: `-txt=`, `-html=`, `-json=`, `-console`, `-bench`, `-core=`, `-coremask=`.
  - Opções novas: `-sensors`, `-log=`, `-membench`, `-diff=`, `-checkupdate[=stable|snapshot|beta|alpha]`.
  - Um `cpuzeez.ini` com as mesmas chaves do `cpuz.ini`.
- **Atualização em um clique.** O app encontra releases novas e instala o pacote certo para o seu SO, depois de conferir o SHA-256.
- **Um único binário nativo**, de cerca de 20 MB (a maior parte é o renderizador e o benchmark de GPU), sem dependências em tempo de execução.

## Screenshots

Geradas com `cpuzeez -screenshots=DIR`, que renderiza cada página e salva um PNG. As do Windows vêm de um AMD Ryzen 9 7900X, MSI PRO X870-P WIFI, 2×16 GB DDR5-6000 e uma RTX 4060, rodando como administrador com o PawnIO. Números de série, MACs e IPs ficam mascarados nesse modo.

### Windows 11

| Sistema (Gerenciador de Tarefas) | O que limita meu FPS? | Benchmarks |
|---|---|---|
| ![Sistema: desempenho](docs/screenshots/windows/system-performance.png) | ![O detector de gargalo](docs/screenshots/windows/bottleneck.png) | ![Benchmarks](docs/screenshots/windows/bench.png) |

| Visão geral | Processador | Memória |
|---|---|---|
| ![Visão geral](docs/screenshots/windows/overview.png) | ![Processador](docs/screenshots/windows/processor.png) | ![Memória e SPD](docs/screenshots/windows/memory.png) |
| **Sensores** | **Teste de estresse** | **Benchmark de memória** |
| ![Sensores](docs/screenshots/windows/sensors.png) | ![Teste de estresse monitorado](docs/screenshots/windows/stress.png) | ![Benchmark de memória e cache](docs/screenshots/windows/membench.png) |
| **Dispositivos** | **Gráficos** | **Placa-mãe** |
| ![Dispositivos](docs/screenshots/windows/devices.png) | ![Gráficos](docs/screenshots/windows/graphics.png) | ![Placa-mãe](docs/screenshots/windows/mainboard.png) |
| **Histórico de benchmarks** | **Comparar relatórios** | **Preferências** |
| ![Histórico](docs/screenshots/windows/history.png) | ![Comparar relatórios](docs/screenshots/windows/compare.png) | ![Preferências](docs/screenshots/windows/settings.png) |
| **Tema claro, em português** | **Sobre, em português** | **Clocks** |
| ![Visão geral no tema claro, em português](docs/screenshots/windows/overview-light.png) | ![Sobre, em português](docs/screenshots/windows/about-pt.png) | ![Clocks](docs/screenshots/windows/clocks.png) |

### Linux

Ubuntu 24.04 (WSLg, Wayland), na mesma máquina, NVIDIA via NVML:

| Visão geral | Sensores | Dispositivos |
|---|---|---|
| ![Visão geral no Linux](docs/screenshots/linux/overview.png) | ![Sensores no Linux](docs/screenshots/linux/sensors.png) | ![Dispositivos no Linux](docs/screenshots/linux/devices.png) |
| **Sistema (Gerenciador de Tarefas)** | **O que limita meu FPS?** | **Processador** |
| ![Sistema no Linux: desempenho](docs/screenshots/linux/system-performance.png) | ![Detector de gargalo no Linux](docs/screenshots/linux/bottleneck.png) | ![Processador no Linux](docs/screenshots/linux/processor.png) |

### macOS

Geradas pelo CI nos runners macOS do GitHub. São máquinas virtuais, então alguns campos mostram o que o hypervisor expõe:

| Apple Silicon: Visão geral | Apple Silicon: Processador | Mac Intel: Visão geral |
|---|---|---|
| ![Visão geral no macOS ARM](docs/screenshots/macos/overview.png) | ![Processador no macOS ARM](docs/screenshots/macos/processor.png) | ![Visão geral no Mac Intel](docs/screenshots/macos-intel/overview.png) |

## Instalação

Cada [release](https://github.com/joaoregis/cpuzeez/releases) traz um instalador por plataforma, o `SHA256SUMS.txt` e a seção do [changelog](CHANGELOG.md) daquela versão:

| Plataforma | Arquivo | Observações |
|---|---|---|
| Windows 10/11 x64 | `cpuzeez-<ver>-windows-x64.msi` | Instala para todos os usuários, cria atalho no menu Iniciar e adiciona o `cpuzeez` ao `PATH` |
| Windows (portátil) | `cpuzeez-<ver>-windows-x64-portable.zip` | Só o `cpuzeez.exe` |
| macOS 11+ (Intel e Apple Silicon) | `cpuzeez-<ver>-macos-universal.dmg` | Arraste o `CpuZeeZ.app` para Aplicativos |
| macOS (instalador) | `cpuzeez-<ver>-macos-universal.pkg` | Instala o app e o `/usr/local/bin/cpuzeez` |
| Debian 12+, Ubuntu 22.04+ | `cpuzeez_<ver>_amd64.deb` / `_arm64.deb` | `sudo apt install ./cpuzeez_<ver>_amd64.deb` |
| Arch Linux | `cpuzeez-bin-<ver>-1-x86_64.pkg.tar.zst` | `sudo pacman -U cpuzeez-bin-*.pkg.tar.zst` |
| Qualquer Linux (glibc 2.35+) | `cpuzeez-<ver>-linux-<arch>.tar.gz` | Binário, arquivo `.desktop` e ícones |

### Por que o Windows ou o macOS mostra um aviso de segurança

**O CPU ZeeZ não é assinado com certificado do publicador.** No Windows, tirar o aviso exige um certificado de assinatura de código. No macOS, exige a assinatura paga do Apple Developer (US$ 99/ano) para a notarização. Os dois custam dinheiro todo ano, e o CPU ZeeZ é um projeto gratuito, ainda sem orçamento para isso. Os executáveis do macOS usam assinatura ad-hoc: ela permite rodar, mas não identifica o desenvolvedor nem vale como notarização.

**O que o aviso significa.** O Windows e o macOS não conseguem saber quem publicou o arquivo. **Não significa que um vírus foi encontrado.** O SmartScreen funciona por reputação e avisa sobre downloads sem assinatura que ainda foram pouco executados.

**Como conferir se o seu download é legítimo:**
1. Toda versão traz o `SHA256SUMS.txt`. Compare o hash do seu arquivo com o da lista:
   - Windows (PowerShell): `Get-FileHash .\cpuzeez-<ver>-windows-x64.msi -Algorithm SHA256`
   - macOS: `shasum -a 256 cpuzeez-<ver>-macos-universal.dmg`
   - Linux: `sha256sum cpuzeez_<ver>_amd64.deb`
2. Toda versão é gerada pelo [workflow de release](.github/workflows/release.yml) público, nos servidores do GitHub, a partir do commit marcado deste código aberto. Os logs da compilação são públicos.
3. O atualizador do app confere o SHA-256 sozinho antes de instalar qualquer coisa ([Atualizações](#atualizações)).

**Como continuar:**
- **Windows:** quando o SmartScreen mostrar "O Windows protegeu o computador", clique em *Mais informações → Executar assim mesmo*.
- **macOS:** o Gatekeeper pode bloquear o app ou o `.pkg` porque a Apple não consegue verificar o desenvolvedor.
  1. Confira o download com o `SHA256SUMS.txt` (acima).
  2. Tente abrir uma vez e feche o aviso.
  3. Vá a *Ajustes do Sistema → Privacidade e Segurança → Abrir Mesmo Assim* e confirme *Abrir*.

  Para o `.pkg`, conclua a instalação e repita para o app se aparecer outro aviso. No macOS 11–12, a opção fica em *Preferências do Sistema → Segurança e Privacidade → Geral*. Clicar com o botão direito → *Abrir* não contorna mais essa verificação no macOS 15 ou posterior. Veja as [instruções da Apple](https://support.apple.com/pt-br/102445).
- **Linux:** pacotes instalados com `apt`, `pacman` ou pelo tarball não mostram esse aviso.

Com Rust instalado, também dá para usar `cargo install --git https://github.com/joaoregis/cpuzeez`.

## Atualizações

**Aviso de atualização.** Ao abrir, o CPU ZeeZ pergunta à API de releases do GitHub se existe versão mais nova. É um único `GET` anônimo; nada sobre o seu computador é enviado.
- Se houver, uma faixa oferece **Baixar e instalar**. Ela baixa o pacote do seu SO e da sua CPU, confere o SHA-256 com o `SHA256SUMS.txt` da release e então o abre:
  - **Windows:** o MSI.
  - **macOS:** o DMG ou o PKG.
  - **Debian/Ubuntu e Arch:** `pkexec` com `apt` ou `pacman`.
  - **Outros:** abre a pasta do download.
- **Novidades** abre as notas da release.
- Também dá para checar em *Sobre ▸ Procurar atualizações*, ou com `cpuzeez -checkupdate`.
- **Canais de atualização.** *Configurações ▸ Atualizações ▸ Canal de atualização* escolhe o que a verificação pode oferecer:
  - **Estável:** só versões finais (o padrão numa versão final).
  - **Snapshot:** também snapshots, prévias com todas as novidades (o padrão quando você roda um snapshot).
  - **Beta:** também betas, partes das novidades em desenvolvimento (o padrão quando você roda uma beta).
  - **Alfa:** tudo o que foi publicado, inclusive código instável.
  A verificação ao abrir segue o canal escolhido, depois que você a permitiu. `cpuzeez -checkupdate=beta` mostra o que um canal ofereceria sem mudar nada. Pré-versões podem estar inacabadas; há um modelo de issue *Beta report* para o que você encontrar.
- Para desligar, use `CheckUpdates=0` no `cpuzeez.ini`.
- O aviso usa o `curl` do sistema, que já vem no Windows 10+, no macOS e nas distros Linux desktop.

**Para atualizar manualmente**, basta instalar a versão nova por cima:

| Plataforma | Como |
|---|---|
| Windows | Rode o `.msi` novo. Ele atualiza no lugar: mesma pasta, um único item em *Aplicativos instalados*, PATH e atalho mantidos. Instalar uma versão mais antiga por cima de uma mais nova é recusado. O CI testa instalação → atualização → bloqueio de downgrade → desinstalação a cada push. |
| macOS | Arraste o `CpuZeeZ.app` novo por cima do antigo, ou rode o `.pkg` novo |
| Debian / Ubuntu | `sudo apt install ./cpuzeez_<ver>_amd64.deb` |
| Arch | `sudo pacman -U cpuzeez-bin-<ver>-1-x86_64.pkg.tar.zst` |

## Desinstalação

O CPU ZeeZ não tem um desinstalador próprio: ele sai do jeito que cada plataforma remove programas. O CI confere a remoção no Windows a cada push (a pasta, o atalho do menu Iniciar e a entrada no PATH somem; suas configurações ficam).

| Plataforma | Como |
|---|---|
| Windows (MSI) | *Configurações ▸ Aplicativos ▸ Aplicativos instalados ▸ CPU ZeeZ ▸ Desinstalar*, ou `msiexec /x` no `.msi`. O driver PawnIO é **mantido**: outros programas (LibreHardwareMonitor, FanControl) o compartilham. Remova-o também em *Aplicativos instalados* se nada mais o usar. |
| Windows (zip portátil) | Apague a pasta. |
| Debian / Ubuntu | `sudo apt remove cpuzeez` |
| Arch | `sudo pacman -R cpuzeez-bin` |
| macOS | Arraste o `CpuZeeZ.app` para a Lixeira (e `/usr/local/bin/cpuzeez`, se usou o `.pkg`). |

**Seus dados** (configurações, sessões, histórico, registros) nunca são apagados por uma desinstalação. Ficam em `%APPDATA%\CpuZeeZ`, `~/Library/Application Support/CpuZeeZ` ou `$XDG_CONFIG_HOME/cpuzeez`; apague à mão, ou antes use *Configurações ▸ Privacidade e dados ▸ Apagar tudo*.

## Acesso privilegiado ao hardware

Como no CPU-Z, algumas informações só vêm de registradores do hardware: MSRs, o SMBus onde ficam as EEPROMs SPD, o chip Super I/O dos sensores, o controlador de memória e o SMU da AMD. O CPU-Z instala o próprio driver de kernel para isso. **O CPU ZeeZ não tem driver próprio.** No Windows ele usa o [PawnIO](#windows-pawnio), um driver assinado e open source que os pacotes trazem como componente opcional; nos outros sistemas, usa o que o SO já oferece. O que não consegue ler, simplesmente omite.

| | Sem privilégios | Com privilégios |
|---|---|---|
| **Windows** | CPU, caches, topologia, DMI, BIOS, lista e links PCI, nomes do chipset, tipo/tamanho/canais da memória, SMART NVMe, rede, bateria, GPUs (NVML/NVAPI/ADL) com temperaturas e clocks, monitores, SO, timers, benchmarks | **+ PawnIO** (como administrador): clock real dos núcleos (APERF/MPERF), P-states, núcleos preferenciais, SPD, timings de memória, voltagens/temperaturas/ventoinhas do Super I/O, temperaturas e potência da CPU, SMU da AMD, temperatura do PCH Intel, temperatura das DIMMs, limites do SMART ATA |
| **Linux** | Tudo acima, mais sensores hwmon (`k10temp`, `coretemp`, `nct6775`, `it87`, `amdgpu`…), clocks via cpufreq, potência RAPL, e SPD quando `ee1004`/`spd5118` estão carregados | **+ root**: MSRs (`modprobe msr`), PCI config completo, SPD via `i2c-dev`, Super I/O via `/dev/port`, SMN/UMC da AMD, MCHBAR Intel, SMART via ioctl, tabela do SMU com o módulo opcional `ryzen_smu` |
| **macOS** | CPU (sysctl / CPUID), caches, clusters, SMBIOS (Intel) ou IODeviceTree (Apple Silicon), tamanho e tipo da memória, PCI (IOKit), GPU e Metal, monitores (EDID), discos, USB, rede, temperaturas/ventoinhas/potência do SMC, saúde da bateria, benchmarks | Nada a mais: o macOS não permite acesso de terceiros a registradores, e o Apple Silicon não tem SPD, MSRs nem Super I/O |

### O que mais exige administrador ou root

Quase tudo no CPU ZeeZ roda como usuário comum. Estas são as exceções, e cada tela avisa quando se aplicam:

| Recurso | Windows | Linux | macOS |
|---|---|---|---|
| **FPS e GPU Busy** (ZeeZ Live) | administrador (o PresentMon lê os eventos do kernel gráfico); a aba Ao vivo oferece o reinício | MangoHud, sem root | Metal HUD, sem root; sem tempos de quadro de outros apps |
| **Rede por processo** | administrador | só TCP; processos de outros usuários exigem root | `nettop`, sem root |
| **Encerrar, suspender ou alterar um processo ou serviço** | um pedido do UAC para aquela ação | `pkexec` para aquela ação | autorização para aquela ação |
| **Itens de inicialização, recuperação e contas de serviços** | um pedido do UAC para aquela ação | `pkexec`; a recuperação é um drop-in do systemd que você pode apagar | autorização para aquela ação |
| **Registradores de hardware** (veja acima) | PawnIO + administrador | root | impossível |

Uma ação que exige direitos roda numa cópia elevada e descartável do CPU ZeeZ, que só aceita uma lista fixa de operações ([modelo de ameaças](docs/security/threat-model.md)). Tudo o que ela faz vai para o registro de auditoria local.

### Windows: PawnIO

O [PawnIO](https://pawnio.eu) é um driver de kernel assinado, open source e programável, também usado pelo LibreHardwareMonitor e pelo FanControl.

Os pacotes para Windows trazem o PawnIO com versão fixada e SHA-256 conferido ([`packaging/pins.txt`](packaging/pins.txt)):

1. Instale:
   - **MSI:** *Hardware access (PawnIO driver)* vem marcado em *Custom Setup*. Desmarque para não instalar o driver. Para instalar em silêncio sem ele, use `msiexec /i cpuzeez-….msi ADDLOCAL=Main`.
   - **No app:** clique em *Instalar acesso ao hardware* no aviso amarelo. O CPU ZeeZ confere o instalador incluído com o hash fixado, e o Windows pede permissão.
   - **Por conta própria:** `winget install namazso.PawnIO`, ou o instalador em [pawnio.eu](https://pawnio.eu).
2. Os módulos oficiais ([PawnIO.Modules](https://github.com/namazso/PawnIO.Modules/releases), LGPL-2.1) vêm na pasta `pawnio-modules` ao lado do `cpuzeez.exe`. Se compilar do código-fonte, extraia-os lá.
3. Rode o CPU ZeeZ **como administrador**. A página *Sobre* (e a seção *Binaries* do relatório) passa a mostrar `Hardware access: PawnIO 2.2.0 (modules: …)`.

Detalhes em [docs/windows-pawnio.md](docs/windows-pawnio.md).

### Linux: interfaces do kernel

```bash
sudo modprobe msr i2c-dev   # MSRs e SMBus
```
```bash
sudo modprobe spd5118       # SPD DDR5 (kernel 6.11+), ou ee1004 para DDR4
```
```bash
sudo cpuzeez                # detalhe completo; sem sudo você ainda vê quase tudo
```

## Qualidade, acessibilidade e segurança

- **Testado sem depender de uma pessoa.** Os leitores de dados e o detector rodam com entradas aleatórias e corrompidas a cada push, e 11 alvos de fuzz rodam agendados (`fuzz/`). Um jogo sintético alimenta um teste longo de **soak** que confere que memória, handles e threads ficam estáveis ([como](docs/testing/soak.md)). Os instaladores são testados no CI: instalar, atualizar a partir de toda versão publicada, recusar downgrade, driver opcional e remoção limpa.
- **Acessível.** O texto atinge o contraste WCAG AA nos dois temas, as cores dos gráficos atingem 3:1, nada depende só de cor (avisos levam um ícone), as páginas se mantêm de 75% a 200% de escala, tabelas e abas funcionam pelo teclado, e widgets e gráficos desenhados expõem nomes e valores a tecnologias assistivas. O que ainda não está coberto está em [docs/accessibility.md](docs/accessibility.md).
- **Segurança.** [Modelo de ameaças](docs/security/threat-model.md), [SECURITY.md](SECURITY.md) para relatar um problema, actions fixadas por commit, tokens de workflow só de leitura, `cargo-deny` e Dependabot. As versões ainda não são assinadas ([ADR 0009](docs/adr/0009-releases-stay-unsigned-for-now.md)); cada uma vem com `SHA256SUMS.txt`.
- **Um guia de auditoria** para quem revisar o projeto: [docs/audit/](docs/audit/README.md).

## Orçamento de desempenho

O CPU ZeeZ roda ao lado do seu jogo, então segue um orçamento: no máximo 2% de um núcleo para o coletor a 1 s com o overlay a 4 Hz, no máximo 150 MB de memória e nenhum pico de tempo de quadro causado pela coleta. Os números medidos e como são medidos estão em [docs/performance.md](docs/performance.md).

## Uso

Rode `cpuzeez` sem argumentos para abrir o app. Todas as opções do CPU-Z funcionam, mais algumas novas:

| Opção | O que faz |
|---|---|
| `-txt=ARQ` | Relatório TXT completo (layout do dump do CPU-Z). O `.txt` é adicionado se faltar |
| `-html=ARQ` | Relatório HTML (com tema claro e escuro) |
| `-json=ARQ` / `-jsontxt=ARQ` | Relatório JSON (a mesma árvore do TXT) |
| `-console` | Imprime o relatório TXT no terminal |
| `-bench` | Benchmark de CPU com uma e com várias threads, salvo em `<hostname>.txt` e no histórico |
| `-membench` | Latência e banda de memória e cache (`-membench=quick` para uma rodada curta) |
| `-sensors` | Imprime todos os sensores uma vez, com o id de cada um |
| `-log=ARQ[,SEGUNDOS]` | Grava todos os sensores em CSV a cada segundo, até SEGUNDOS ou Ctrl+C |
| `-diff=ANTIGO.json,NOVO.json` | Compara dois relatórios (`NOVO` pode ser `now`; acrescente `,all` para incluir valores ao vivo) |
| `-core=N` | Mostra o clock do núcleo N e sai |
| `-coremask=HEX` | Restringe o `-bench` a essas CPUs lógicas |
| `-checkupdate[=canal]` | Verifica se há release mais nova no GitHub (`stable`, `snapshot`, `beta` ou `alpha`; sem valor, o canal das Configurações) |
| `-screenshots=DIR` | Renderiza cada página em PNG e sai (para docs e CI) |

Atalhos da interface, como no CPU-Z:
- **F5** salva a página atual em `.bmp`.
- **F6** copia a página para a área de transferência.
- **F7** salva um arquivo `.cvf` (o nosso relatório JSON).

### `cpuzeez.ini` e preferências

O `cpuzeez.ini` fica ao lado do executável e guarda as chaves de detecção de hardware. Ele usa as mesmas chaves do `cpuz.ini`, e o arquivo UTF-16 do próprio CPU-Z é lido sem conversão:

```ini
[CPU-Z]
PCI=1          ; 0 também desliga chipset, SPD e SMBus
DMI=1
Sensor=1
SMBus=1
SPD=1
LPCIO=1
Display=1
UseDisplayAPI=1 ; NVML / NVAPI / ADL
SMART=1
CheckUpdates=1  ; aviso de atualização ao abrir
```

As preferências da interface ficam na pasta de dados do usuário e são editadas na página **Preferências**:
- idioma, tema, unidades, ícones da bandeja, fechar para a bandeja, mini monitores e seus layouts;
- intervalo de leitura dos sensores, histórico dos gráficos, temperatura de alerta, alertas locais.

A pasta é `%APPDATA%\CpuZeeZ`, `~/Library/Application Support/CpuZeeZ` ou `~/.config/cpuzeez`. Ela também guarda o histórico de benchmarks (`history.json`) e os logs de sensores.

Para mapear os sensores de uma placa-mãe nova, `CPUZEEZ_EC_DUMP=1 cpuzeez -txt=placa` inclui no relatório os registradores brutos do Super I/O e as entradas sem mapa. Veja [docs/boards.md](docs/boards.md).

## Cobertura de funcionalidades

A especificação completa, montada a partir do relatório e do readme do próprio CPU-Z, está em [FEATURES.md](FEATURES.md). O que mudou em cada versão está no [CHANGELOG.md](CHANGELOG.md).

Legenda:
- ✅ implementado e verificado na máquina de referência (Windows nativo; Linux no WSL2, que só expõe o que uma VM enxerga)
- ☑️ implementado e compilado/testado no CI naquele SO, ainda não conferido em hardware real
- 🔑 requer administrador/root (veja acima)
- ❌ impossível naquela plataforma

| Funcionalidade | Windows | Linux | macOS |
|---|---|---|---|
| Nome do processador, fabricante, família/modelo/stepping, marca, conjuntos de instruções | ✅ | ✅ | ✅ (CI; ARM via sysctl / MIDR) |
| Codinome, pacote, processo, nome do stepping, TDP, Tjmax | ✅ | ✅ | ✅ (CI) |
| Árvore de topologia, CCD/CCX, núcleos híbridos P/E/LP, grupos de CPU (>64 threads) | ✅ | ✅ | ☑️ (níveis de desempenho) |
| Caches: tamanho, vias, linha, instâncias, compartilhamento | ✅ | ✅ | ✅ (CI) |
| Dumps por thread: CPUID bruto e MSRs | ✅ (MSR 🔑) | ✅ (MSR 🔑) | ☑️ (x86) |
| P-states, núcleos preferenciais (CPPC), BCLK medido | ✅🔑 | ☑️🔑 | ❌ |
| Clock dos núcleos: APERF/MPERF, ou do SO, ou medido | ✅🔑 / medido | ✅ medido / ☑️ cpufreq | ✅ medido (CI) |
| SMU da AMD: temperaturas por núcleo/L3/SoC, trilhos VDDCR, PPT/TDC/EDC, FCLK/UCLK/MCLK, Fmax, max ratio por núcleo | ✅🔑 (Zen 4 verificado) | ☑️🔑 (`ryzen_smu`) | ❌ |
| Temperaturas da CPU, VID, potência do pacote/núcleos (RAPL), clocks por núcleo | ✅🔑 | ☑️ hwmon / 🔑 | ☑️ SMC |
| Timers: ACPI, perf, sistema | ✅ | ✅ | ☑️ |
| BIOS: UEFI, data, AGESA / microcode | ✅ | ✅ | ☑️ (iBoot) |
| DMI / SMBIOS: todos os tipos que o CPU-Z mostra | ✅ | ☑️🔑 (parcial sem root) | ☑️ Intel / sintetizado no ARM |
| Chipset: northbridge/southbridge + revisão, bus specification, link da interface gráfica | ✅ | ☑️ | ☑️ |
| Tipo, tamanho e canais da memória | ✅ | ✅ | ☑️ (incl. Apple Silicon) |
| Frequência da memória, CL/tRCD/tRP/tRAS/tRC/tRFC, CR, uncore/fabric | ✅🔑 AMD · ☑️🔑 Intel | ☑️🔑 | ❌ |
| SPD: todos os campos, tabelas JEDEC/XMP/EXPO, dump hex | ✅🔑 | ☑️ (`spd5118`/`ee1004`) / 🔑 | ❌ (memória soldada) |
| Temperaturas das DIMMs | ✅🔑 | ☑️ | ❌ |
| LPCIO: fabricante, modelo, IDs e registradores de configuração do Super I/O | ✅🔑 | ☑️🔑 | ❌ |
| Sensores da placa-mãe: voltagens, temperaturas, ventoinhas | ✅🔑 (NCT6687 verificado) | ☑️ hwmon | ☑️ SMC |
| Temperatura do chipset | ✅🔑 (AMD via Super I/O) · ☑️ PCH Intel | ☑️ | ❌ |
| GPU: especificações, clocks, limites de potência/temperatura, VRAM e fabricante, link, driver | ✅ (NVML, NVAPI, ADL, DXGI) | ✅ (DRM, NVML) | ☑️ (Metal) |
| Sensores da GPU: temperatura, hot spot, junção da memória, ventoinhas, potência, carga, limitação | ✅ | ☑️ | ❌ |
| Monitores (EDID) | ✅ | ✅ | ☑️ |
| Lista de dispositivos PCI, IDs, nomes, links | ✅ | ✅ | ☑️ |
| Decodificação de capabilities PCI e dump hex do config | ❌¹ | ✅ (completo com root) | ❌ |
| Discos: modelo, serial, firmware, capacidade, barramento, link, volumes | ✅ | ✅ | ☑️ |
| SMART: saúde, atributos, temperatura, desgaste, horas, dados lidos/gravados | ✅ (limites ATA 🔑) | ☑️🔑 | ☑️ (`smartctl` se instalado) |
| Saúde da bateria: capacidade de projeto × total, desgaste, ciclos | ☑️ | ☑️ | ☑️ |
| Adaptadores de rede: link, MAC, IPs, gateway, DNS, driver, Wi-Fi | ✅ | ✅ | ☑️ |
| Dispositivos USB, NPU / dispositivos de IA | ✅ | ✅ | ☑️ |
| Software: SO, edição, build, data de instalação, DirectX / Vulkan / Metal | ✅ | ✅ | ☑️ |
| Página de sensores, log CSV, detecção de limitação, bandeja / mini monitor | ✅ | ✅ (mini monitor; a bandeja requer um host StatusNotifier) | ☑️ |
| Benchmarks: suíte de CPU, memória, armazenamento e latência entre núcleos, histórico, teste de estabilidade | ✅ | ✅ | ✅ (CI; latência como dispersão estatística) |
| Benchmark de GPU (wgpu) | ✅ (DX12, Vulkan) | ☑️ (Vulkan) | ☑️ (Metal) |
| Saúde dos discos: autotestes SMART, varredura de superfície, resistência | ✅🔑 | ☑️🔑 | ❌ autotestes · ☑️🔑 varredura |
| HDR / faixa de atualização dos monitores, formatos de áudio, Thunderbolt/USB4, bateria Bluetooth | ✅ | ☑️ | ☑️ |
| Relatórios (TXT/HTML/JSON), comparação de relatórios, F5/F6/F7 | ✅ | ✅ | ✅ (CI) |

¹ Nenhum módulo do PawnIO dá acesso genérico ao PCI config no Windows. IDs, nomes e links dos dispositivos continuam vindo do SO.

### Fora de propósito

- **Validação online** (valid.x86.fr, Validator Statistics). É um serviço da CPUID, então o CPU ZeeZ não envia para lá. Compartilhe o relatório HTML/JSON.
- **O algoritmo e os scores de referência do benchmark do CPU-Z.** São trabalho da CPUID. Os benchmarks do CPU ZeeZ são próprios, e os scores não são comparáveis com os do CPU-Z.
- **Alvos obsoletos:** Itanium/IA-64, Windows 9x e CPUs anteriores ao i686. Nenhum dos SOs atuais roda neles.

## Precisão: lado a lado com o CPU-Z

Na máquina de referência (Ryzen 9 7900X, MSI PRO X870-P WIFI, Windows 11 25H2), o `cpuzeez -txt` foi comparado seção por seção com o `cpuz.exe -txt`.

**Saíram iguais ao CPU-Z:**
- **CPU e topologia:**
  - Árvore APIC, conjuntos de instruções, caches, microcode.
  - Codinome, pacote e stepping (`Raphael`, `Socket AM5 (LGA1718)`, `RPL-B2`).
  - P-states (`47.00x - 1.256 V`), núcleos preferenciais.
- **SMU da AMD:**
  - Versão `84.79.225`, `PMTV 00540004`, PPT/TDC/EDC 230 W / 160 A / 225 A, `Max Frequency 5700 MHz`.
  - Trilhos VDDCR_VDD/SOC/MISC e o layout das temperaturas por núcleo, L3 e SoC.
- **Chipset e memória:**
  - `Bus Specification PCI-Express 5.0 (32.0 GT/s)`.
  - Timings DDR5 `30-36-36-76`, tRC 112, razão 1:30, `Memory Max Frequency 2800.0 MHz`.
- **SPD:** os 2 × 1024 bytes lidos ao vivo são idênticos byte a byte aos do CPU-Z, assim como todos os campos decodificados.
- **Super I/O:** registradores, voltagens, temperaturas e ventoinhas do NCT6687 (`Fan 15 SYS 2`, `Fan 16 SYS 1`).
- **DMI, discos, USB, GPUs e software:**
  - RTX 4060 `AD107-400`, 3072 cores, fabricante da memória `Samsung`, limites e clocks.
  - O EDID do monitor e a linha do SO.
- **Timers.**

**Diferenças intencionais:**
- **Tamanho da ROM do BIOS:** mostramos `32768 KB`, o campo estendido do SMBIOS; o CPU-Z o ignora e mostra `16384 KB`.
- **TMUs da RTX 4060:** mostramos `96`, a especificação pública (24 SMs × 4); o CPU-Z mostra 192.
- **Valores ao vivo** dependem do momento: temperaturas, clocks e max ratio por núcleo. Os ratios são medidos como o CPU-Z descreve (cada núcleo carregado sozinho), então variam com a temperatura.
- **Clock de referência:** medido por TSC ÷ razão base. Dá `Memory Frequency 2999.9 MHz`, onde o CPU-Z mostra `2998.8 MHz`.

**Não decodificado:**
- Os limites "fused" que o CPU-Z obtém por uma chamada de mailbox do SMU (`fused: …`) e o escalar do PBO.
- A decodificação de capabilities PCI no Windows.

## Processo clean room

O CPU ZeeZ foi construído para que nenhuma expressão do CPU-Z (código, tabelas ou texto) pudesse acabar neste projeto:

1. **Observação de caixa-preta.** A opção documentada `-txt` do CPU-Z gerou o relatório dele numa máquina de teste. Esse relatório mostra *o que* o CPU-Z exibe e em que formato, não *como* ele faz.
2. **Inventário de funcionalidades.** O readme e os nomes de strings e classes RTTI exportados pelo `cpuz.exe`, inspecionados com o [REA](https://github.com/morluto/rea) e o Ghidra, mostram *quais* funcionalidades e famílias de hardware existem. **Nenhuma função foi descompilada ou traduzida.**
3. **Especificação.** Tudo isso virou o [FEATURES.md](FEATURES.md).
4. **Implementação só com fontes públicas:**
   - CPU e firmware: Intel SDM e datasheets, AMD APM/PPR, DMTF SMBIOS.
   - Memória: anexos SPD da JEDEC, JEP106, layouts do XMP da Intel e do EXPO da AMD.
   - Barramentos e telas: especificações PCI/PCIe, VESA EDID.
   - Discos: NVMe e SMART ATA.
   - Chips e GPUs: datasheets dos chips e documentação de NVML, ADL, DXGI, IOKit e sysfs.
   - Documentação da comunidade para interfaces não documentadas mas públicas (layout das células da tabela PM da AMD, IDs de função da NVAPI, rótulos de sensores das placas), usada como fato e reescrita no nosso código.
5. **Verificação por comportamento.** A saída foi comparada com o relatório do CPU-Z no mesmo hardware. Onde há diferença, seguimos a especificação pública.

## Compilando

Requer Rust 1.89+ (`rustup`). Não há bibliotecas de sistema para instalar, nem no Linux: a interface carrega X11/Wayland/OpenGL em tempo de execução, e o ícone da bandeja no Linux fala D-Bus em Rust puro.

```bash
git clone https://github.com/joaoregis/cpuzeez
```
```bash
cargo build --release
```
```bash
cargo test
```
```bash
cargo build --release --no-default-features
```

O último comando compila só a linha de comando, sem a interface.

CI ([`ci.yml`](.github/workflows/ci.yml)):
- **Runners nativos:** Windows, Ubuntu x64/arm64 e macOS arm64/x64. Cada job roda rustfmt, clippy e os testes, e depois o próprio binário (todos os formatos de relatório, `-core`, `-bench`, screenshots da interface).
- **Containers:** Debian, Ubuntu 22.04 e Arch Linux.
- **Teste do MSI:** instalação, atualização no lugar, bloqueio de downgrade e desinstalação.

Release ([`release.yml`](.github/workflows/release.yml)): enviar uma tag `v*` gera o MSI (WiX), o DMG/PKG (binário universal, assinaturas conferidas), o `.deb` (cargo-deb) e o pacote do Arch (makepkg). Eles vão para uma release do GitHub cujas notas são a seção daquela versão no [CHANGELOG](CHANGELOG.md).

## Arquitetura

Um workspace Cargo ([ADR 0011](docs/adr/README.md)):

```
crates/core/       lógica pura, sem chamadas ao SO, com testes em todo lugar:
                   árvore do relatório, Metric Engine, layouts do Widget Engine (visão geral,
                   bandeja, mini monitores), decodificadores de SPD/EDID/SMART/Identify,
                   saúde da montagem, alertas, unidades, níveis de detalhe
crates/platform/   adaptadores por SO: CPUID, acesso privilegiado (PawnIO / interfaces do kernel
                   Linux), contadores de disco/rede/memória/motores da GPU, monitores, firmware,
                   partições, C-states, E/S sem cache, páginas grandes, criptografia
src/               o app:
├── main.rs        linha de comando (opções do CPU-Z + -sensors/-log/-membench/-diff), Ctx, ordem das seções
├── cpu/  gpu/  pci/  spd/  sensors/  chipset.rs  smbios.rs  storage.rs  usb.rs  …   leituras
├── telemetry.rs   amostragem no Metric Engine: séries, CSV, detecção de limitação
├── benchkit.rs    princípios dos benchmarks (condições, N execuções, pontuação 1000), tarefas em segundo plano
├── bench.rs  cpusuite.rs  membench.rs  memplus.rs  gpubench.rs  diskbench.rs  latency.rs
├── memtest.rs  drivetest.rs  history.rs  diff.rs  update.rs  executor.rs (ações auditadas)
└── gui/           interface egui: páginas, telas de benchmark, bandeja, mini monitores,
                   editor de layout, paleta de comandos, i18n (en / pt-BR / es)
```

Cada leitura devolve seções de relatório. A interface, os geradores TXT/HTML/JSON e o `-console` leem essa mesma árvore, então o app e o relatório sempre concordam. O plano da 1.0 e seu roadmap estão em [docs/v1/](docs/v1/README.md).

## Contribuindo

Issues e PRs são bem-vindos, principalmente:
- **Placas-mãe novas:** use o [modelo de issue de sensores](.github/ISSUE_TEMPLATE/board-sensors.yml) com a saída de `CPUZEEZ_EC_DUMP=1 cpuzeez -txt=placa`.
- **CPUs e GPUs que faltam nas tabelas** (`src/cpu/db.rs`, `src/gpu/db.rs`), citando fonte pública.
- **Relatórios de Macs reais e de sistemas Intel**, principalmente para os decodificadores de timings de memória Intel.
- **Traduções** (tabelas `PT_BR` / `ES` no fim de cada `src/gui/*.rs`; um idioma novo é uma tabela por arquivo).

Mantenha a regra clean room: **não contribua nada derivado do binário do CPU-Z**. Trabalhe a partir de documentação pública.

Obrigado a [@vinicius741](https://github.com/vinicius741) pelas correções de assinatura no macOS e de memória no Apple Silicon ([#1](https://github.com/joaoregis/cpuzeez/pull/1), [#2](https://github.com/joaoregis/cpuzeez/pull/2)).

## Licença

Licenciado sob [Apache License 2.0](LICENSE-APACHE) ou [MIT license](LICENSE-MIT), à sua escolha.

O PawnIO e seus módulos são projetos separados, com licenças próprias. Os pacotes para Windows os redistribuem sem modificação; veja [THIRD_PARTY.md](THIRD_PARTY.md).
