package com.patenthub.app;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.EditText;
import android.widget.ProgressBar;

public class MainActivity extends Activity {
    private WebView webView;
    private ProgressBar progressBar;
    private static final String PREFS = "patent_hub_prefs";
    private static final String KEY_SERVER = "server_url";
    private static final String DEFAULT_SERVER = "http://192.168.1.100:3000";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        requestWindowFeature(Window.FEATURE_NO_TITLE);

        // 状态栏颜色
        getWindow().setStatusBarColor(0xFF0D1117);

        setContentView(R.layout.activity_main);
        webView = findViewById(R.id.webview);
        progressBar = findViewById(R.id.progress);

        setupWebView();

        String serverUrl = getServerUrl();
        if (serverUrl.isEmpty()) {
            showServerDialog();
        } else {
            loadServer(serverUrl);
        }
    }

    private void setupWebView() {
        WebSettings settings = webView.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(true);
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_ALWAYS_ALLOW);
        settings.setUseWideViewPort(true);
        settings.setLoadWithOverviewMode(true);
        settings.setCacheMode(WebSettings.LOAD_DEFAULT);

        webView.setWebViewClient(new WebViewClient() {
            @Override
            public void onReceivedError(android.webkit.WebView view, int errorCode,
                    String description, String failingUrl) {
                // 连接失败，提示重新输入地址
                showServerDialog();
            }
        });

        webView.setWebChromeClient(new WebChromeClient() {
            @Override
            public void onProgressChanged(WebView view, int newProgress) {
                if (newProgress < 100) {
                    progressBar.setVisibility(View.VISIBLE);
                    progressBar.setProgress(newProgress);
                } else {
                    progressBar.setVisibility(View.GONE);
                }
            }
        });
    }

    private void showServerDialog() {
        EditText input = new EditText(this);
        input.setHint("例如: http://192.168.1.100:3000");
        input.setText(getServerUrl().isEmpty() ? DEFAULT_SERVER : getServerUrl());
        input.setTextColor(0xFF000000);
        input.setPadding(40, 20, 40, 20);

        new AlertDialog.Builder(this)
            .setTitle("Patent Hub 服务器地址")
            .setMessage("请输入电脑上运行的 Patent Hub 地址\n\n提示：启动 Patent Hub 后查看控制台的 Mobile access 地址")
            .setView(input)
            .setCancelable(false)
            .setPositiveButton("连接", (d, w) -> {
                String url = input.getText().toString().trim();
                if (!url.startsWith("http")) url = "http://" + url;
                if (url.endsWith("/")) url = url.substring(0, url.length() - 1);
                saveServerUrl(url);
                loadServer(url);
            })
            .setNeutralButton("重置", (d, w) -> {
                saveServerUrl("");
                showServerDialog();
            })
            .show();
    }

    private void loadServer(String serverUrl) {
        webView.loadUrl(serverUrl + "/search");
    }

    private String getServerUrl() {
        return getSharedPreferences(PREFS, MODE_PRIVATE)
            .getString(KEY_SERVER, "");
    }

    private void saveServerUrl(String url) {
        getSharedPreferences(PREFS, MODE_PRIVATE)
            .edit().putString(KEY_SERVER, url).apply();
    }

    @Override
    public void onBackPressed() {
        if (webView.canGoBack()) {
            webView.goBack();
        } else {
            super.onBackPressed();
        }
    }
}
