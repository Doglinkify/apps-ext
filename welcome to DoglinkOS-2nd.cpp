#include <iostream>

using namespace std;
int main() 
{
   int choice
   cout <<Welcome to DoglinkOS-2nd<< endl;//三种语言
   cout <<欢迎来到DoglinkOS-2nd<< endl;
   cout <<歡迎來到DoglinkOS-2nd<< endl;
   
    
    while(true) {
        cout << "\n请选择操作：" << endl;
        cout << "1. Knowabout DoglinkOS-2nd 了解DoglinkOS-2nd 瞭解DoglinkOS-2nd" << endl;
        cout << "2. Developer's Guide 开发者指南 開發者指南" << endl;
        cout << "3. Example of app 程序示例 程序示例" << endl;
        cout << "4. Exit 退出 退出 "<< endl;
        cout << "choice （1-4）请输入选项 (1-4) 請輸入選項 （1-4）: ";
        
        cin >> choice;
        
        switch(choice) {
            case 1:
                cout <<"Doglink由ruster开发整个系统完全支持gpl v3.0协议是一款极具现代化的系统"<< endl;
                cout <<""<< endl;
                cout <<""<< endl;
                break;
                
            case 2:
                cout <<""<< endl;
                cout <<""<< endl;
                cout <<""<< endl;
                cout <<""<< endl;
                break;

            case 3:
                cout <<""<< endl;
                cout <<""<< endl;
                cout <<""<< endl;
                cout <<""<< endl;
                break;
                
            case 4:
                return 0;
                
            default:
                cout << "\n无效选项，请重新输入 (1-3)!" << endl;
                cin.clear();
                cin.ignore(10000, '\n');
        }
    }
}