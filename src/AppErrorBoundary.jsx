import React from "react";

export class AppErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { error: null, componentStack: "" };
  }

  static getDerivedStateFromError(error) {
    return { error };
  }

  componentDidCatch(error, info) {
    this.setState({ componentStack: info?.componentStack || "" });
    console.error("应用界面渲染失败", error, info);
  }

  render() {
    const { error, componentStack } = this.state;
    if (!error) return this.props.children;
    const diagnostic = [error?.stack || error?.message || `${error}`, componentStack]
      .filter(Boolean)
      .join("\n");
    return (
      <main className="fatal-error-shell" role="alert">
        <section className="fatal-error-card">
          <span className="eyebrow">应用已安全停止当前操作</span>
          <h1>页面遇到错误，没有继续执行迁移</h1>
          <p>
            当前界面无法正常显示。尚未触发的数据库写入不会执行，可以重新加载应用后继续排查。
          </p>
          <div className="fatal-error-summary">
            <strong>错误摘要</strong>
            <code>{error?.message || "未知界面错误"}</code>
          </div>
          <details>
            <summary>查看技术信息</summary>
            <pre>{diagnostic}</pre>
          </details>
          <button
            type="button"
            className="button button--primary"
            onClick={() => window.location.reload()}
          >
            重新加载应用
          </button>
        </section>
      </main>
    );
  }
}
