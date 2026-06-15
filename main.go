package main

import (
	"embed"
	"log"
	"os"
	"path/filepath"

	"Oj-Agent/llm"
	"Oj-Agent/storage"

	"github.com/wailsapp/wails/v3/pkg/application"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	llmClient, err := llm.NewClient(nil)
	if err != nil {
		log.Printf("LLM init error: %v", err)
	}

	if llmClient != nil && llmClient.Available() {
		model := os.Getenv("DEEPSEEK_MODEL")
		if model == "" {
			model = os.Getenv("LLM_MODEL")
		}
		if model == "" {
			model = "deepseek-chat"
		}
		log.Printf("LLM: %s ready", model)
	} else {
		log.Println("LLM: no API key configured — user will be prompted in UI")
	}

	dbPath := filepath.Join(os.TempDir(), "oj-agent.db")
	db, err := storage.Open(dbPath)
	if err != nil {
		log.Printf("DB init error: %v (sessions will not be persisted)", err)
	}

	chatService := NewChatService(llmClient, db)

	app := application.New(application.Options{
		Name:        "OJ Agent",
		Description: "输入算法题目，自动生成题解动画",
		Services: []application.Service{
			application.NewService(chatService),
		},
		Assets: application.AssetOptions{
			Handler: application.AssetFileServerFS(assets),
		},
		Mac: application.MacOptions{
			ApplicationShouldTerminateAfterLastWindowClosed: true,
		},
	})

	app.OnShutdown(func() {
		if db != nil {
			_ = db.Close()
			log.Println("DB closed")
		}
	})

	app.Window.NewWithOptions(application.WebviewWindowOptions{
		Title:            "OJ Agent - 算法题解动画",
		Width:            1400,
		Height:           850,
		MinWidth:         1000,
		MinHeight:        600,
		BackgroundColour: application.NewRGB(10, 14, 23),
		URL:              "/",
	})

	err = app.Run()
	if err != nil {
		log.Fatal(err)
	}
}

func maskKey(key string) string {
	if len(key) <= 8 {
		return "***"
	}
	return key[:4] + "****" + key[len(key)-4:]
}
