import UIKit
import WebKit

class ViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()

        // 启动 Rust 服务器
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
        let dbPath = docs.appendingPathComponent("patent_hub.db").path
        patent_hub_start_server(dbPath)

        // 等待服务器启动后加载 WebView
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            let webView = WKWebView(frame: self.view.bounds)
            webView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            self.view.addSubview(webView)
            webView.load(URLRequest(url: URL(string: "http://127.0.0.1:3000/search")!))
        }
    }
}
