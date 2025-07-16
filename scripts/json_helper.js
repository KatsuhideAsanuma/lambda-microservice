#!/usr/bin/env node

// jqコマンドの代替として使用するNode.jsベースのJSONパーサー
const fs = require('fs');
const path = require('path');

// コマンドライン引数の解析
function parseArgs(args) {
    const options = {
        expression: null,
        silent: false,
        exit: false,
        raw: false
    };
    
    let i = 0;
    while (i < args.length) {
        const arg = args[i];
        if (arg === '-e' || arg === '--exit-status') {
            options.exit = true;
        } else if (arg === '-r' || arg === '--raw-output') {
            options.raw = true;
        } else if (arg === '-s' || arg === '--silent') {
            options.silent = true;
        } else if (!options.expression) {
            options.expression = arg;
        }
        i++;
    }
    
    return options;
}

// JSON式の評価
function evaluateExpression(data, expression) {
    try {
        // 基本的なjq式をJavaScriptに変換
        let jsExpression = expression;
        
        // .status == "ok" -> data.status === "ok"
        jsExpression = jsExpression.replace(/\.(\w+)/g, 'data.$1');
        jsExpression = jsExpression.replace(/==/g, '===');
        jsExpression = jsExpression.replace(/!=/g, '!==');
        
        // .functions -> data.functions
        jsExpression = jsExpression.replace(/^\.(\w+)$/, 'data.$1');
        
        // .functions | length -> data.functions.length
        jsExpression = jsExpression.replace(/data\.(\w+) \| length/, 'data.$1.length');
        
        // -r オプション処理用
        if (expression.includes('-r')) {
            const match = expression.match(/\.(\w+)/);
            if (match) {
                return data[match[1]];
            }
        }
        
        // 単純な値の取得
        if (expression.startsWith('.') && !expression.includes('==') && !expression.includes('length')) {
            const field = expression.substring(1);
            return data[field];
        }
        
        // 式の評価
        const result = eval(jsExpression);
        return result;
    } catch (error) {
        return null;
    }
}

// メイン処理
function main() {
    const args = process.argv.slice(2);
    const options = parseArgs(args);
    
    if (!options.expression) {
        console.error('Usage: node json_helper.js [options] expression');
        process.exit(1);
    }
    
    let input = '';
    
    // 標準入力からJSONを読み取り
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', (chunk) => {
        input += chunk;
    });
    
    process.stdin.on('end', () => {
        if (input.trim() === '') {
            console.error('Error: No input provided');
            process.exit(1);
        }
        try {
            const data = JSON.parse(input);
            const result = evaluateExpression(data, options.expression);
            
            if (options.exit) {
                // -e オプション: 真偽値を終了コードに変換
                process.exit(result ? 0 : 1);
            } else if (options.raw) {
                // -r オプション: 生の値を出力
                console.log(result);
            } else {
                // 通常の出力
                if (result !== null && result !== undefined) {
                    if (typeof result === 'object') {
                        console.log(JSON.stringify(result, null, 2));
                    } else {
                        console.log(result);
                    }
                }
            }
        } catch (error) {
            if (!options.silent) {
                console.error('Error parsing JSON:', error.message);
            }
            process.exit(1);
        }
    });
}

main();
