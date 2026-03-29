import { webview } from '@kit.ArkWeb';
import { router } from '@kit.ArkUI';
import { common } from '@kit.AbilityKit';

const SERVER_URL: string = 'http://127.0.0.1:3000/search';

@Entry
@Component
struct Index {
  controller: webview.WebviewController = new webview.WebviewController();
  @State isLoading: boolean = true;
  @State hasError: boolean = false;
  @State errorMessage: string = '';
  @State progress: number = 0;

  aboutToAppear(): void {
    // 初始化 Web 引擎
    webview.WebviewController.initializeWebEngine();
  }

  /**
   * 重新加载页面
   */
  private reload(): void {
    this.hasError = false;
    this.isLoading = true;
    this.errorMessage = '';
    try {
      this.controller.loadUrl(SERVER_URL);
    } catch (e) {
      this.hasError = true;
      this.errorMessage = '无法加载页面，请确认服务已启动';
    }
  }

  /**
   * 处理返回键：优先在 WebView 内后退，否则退出应用
   */
  onBackPress(): boolean {
    try {
      if (this.controller.accessBackward()) {
        this.controller.backward();
        return true; // 已处理，不退出
      }
    } catch (e) {
      // controller 未就绪，允许默认行为
    }
    return false; // 未处理，执行默认退出
  }

  @Builder
  LoadingIndicator() {
    Column() {
      LoadingProgress()
        .width(48)
        .height(48)
        .color('#58a6ff')
      Text('正在加载 Patent Hub...')
        .fontSize(14)
        .fontColor('#8b949e')
        .margin({ top: 12 })
    }
    .width('100%')
    .height('100%')
    .justifyContent(FlexAlign.Center)
    .alignItems(HorizontalAlign.Center)
    .backgroundColor('#0d1117')
  }

  @Builder
  ErrorPage() {
    Column() {
      Text('⚠')
        .fontSize(64)
        .margin({ bottom: 16 })

      Text('页面加载失败')
        .fontSize(20)
        .fontWeight(FontWeight.Bold)
        .fontColor('#f85149')
        .margin({ bottom: 8 })

      Text(this.errorMessage || '请检查本地服务是否已启动')
        .fontSize(14)
        .fontColor('#8b949e')
        .textAlign(TextAlign.Center)
        .margin({ bottom: 24 })
        .padding({ left: 32, right: 32 })

      Button('重新加载')
        .fontSize(16)
        .fontColor('#ffffff')
        .backgroundColor('#238636')
        .borderRadius(8)
        .width(160)
        .height(44)
        .onClick(() => {
          this.reload();
        })

      Text('请确认 Patent Hub 服务运行在\nhttp://127.0.0.1:3000')
        .fontSize(12)
        .fontColor('#484f58')
        .textAlign(TextAlign.Center)
        .margin({ top: 32 })
    }
    .width('100%')
    .height('100%')
    .justifyContent(FlexAlign.Center)
    .alignItems(HorizontalAlign.Center)
    .backgroundColor('#0d1117')
  }

  build() {
    Stack() {
      // WebView 主体
      Web({ src: SERVER_URL, controller: this.controller })
        .width('100%')
        .height('100%')
        .javaScriptAccess(true)
        .domStorageAccess(true)
        .mixedMode(MixedMode.All)
        .darkMode(WebDarkMode.Auto)
        .cacheMode(CacheMode.Default)
        .overviewModeAccess(true)
        .fileAccess(true)
        .imageAccess(true)
        .zoomAccess(false)
        .onProgressChange((event) => {
          if (event) {
            this.progress = event.newProgress;
          }
        })
        .onPageBegin(() => {
          this.isLoading = true;
          this.hasError = false;
        })
        .onPageEnd(() => {
          this.isLoading = false;
        })
        .onErrorReceive((event) => {
          if (event && event.error) {
            const errorCode = event.error.getErrorCode();
            const errorInfo = event.error.getErrorInfo();
            // 连接错误时显示错误页
            if (errorCode === -2 || errorCode === -6 || errorCode === -7) {
              this.hasError = true;
              this.errorMessage = `连接失败 (${errorCode}): ${errorInfo}`;
              this.isLoading = false;
            }
          }
        })
        .onHttpErrorReceive((event) => {
          if (event && event.response) {
            const statusCode = event.response.getResponseCode();
            if (statusCode >= 500) {
              this.hasError = true;
              this.errorMessage = `服务器错误 (${statusCode})`;
              this.isLoading = false;
            }
          }
        })

      // 加载进度条
      if (this.isLoading && !this.hasError) {
        Column() {
          Progress({ value: this.progress, total: 100, type: ProgressType.Linear })
            .width('100%')
            .height(3)
            .color('#58a6ff')
            .backgroundColor('#21262d')
        }
        .width('100%')
        .position({ x: 0, y: 0 })
      }

      // 首次加载指示器
      if (this.isLoading && this.progress === 0 && !this.hasError) {
        this.LoadingIndicator()
      }

      // 错误页面
      if (this.hasError) {
        this.ErrorPage()
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#0d1117')
  }
}
