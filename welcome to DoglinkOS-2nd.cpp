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
        cout << "2. 开发者指南" << endl;
        cout << "3. 退出" << endl;
        cout << "请输入选项 (1-3): ";
        
        cin >> choice;
        
        switch(choice) {
            case 1:
                cout <<""<< endl;
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
                return 0;
                
            default:
                cout << "\n无效选项，请重新输入 (1-3)!" << endl;
                cin.clear();
                cin.ignore(10000, '\n');
        }
    }
}