
#include <iostream>
#include <vector>
#include <cstdlib>
#include <ctime>

// 使用二维向量代表游戏板
std::vector<std::vector<int>> board(4, std::vector<int>(4, 0));






// 在随机空位生成一个2或4
void add_random_tile() {
    std::vector<std::pair<int, int>> empty_cells;
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            if (board[i][j] == 0) {
                empty_cells.push_back({i, j});
            }
        }
    }
    if (!empty_cells.empty()) {
        int index = rand() % empty_cells.size();
        int row = empty_cells[index].first;
        int col = empty_cells[index].second;
        board[row][col] = (rand() % 2 == 0) ? 2 : 4;
    }
}

// 打印游戏板
void print_board() {
    system("clear"); // 清屏
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            if (board[i][j] == 0) {
                std::cout << "    .";
            } else {
                printf("%5d", board[i][j]);
            }
        }
        std::cout << std::endl << std::endl;
    }
    std::cout << std::endl;
}

// 初始化游戏
void initialize_game() {
    srand(time(0));
    add_random_tile();
    add_random_tile();
}




// 向左移动
bool move_left() {
    bool moved = false;
    for (int i = 0; i < 4; ++i) {
        std::vector<int> new_row;
        for (int j = 0; j < 4; ++j) {
            if (board[i][j] != 0) {
                new_row.push_back(board[i][j]);
            }
        }
        for (int j = 0; j < new_row.size() - 1; ++j) {
            if (new_row[j] == new_row[j+1]) {
                new_row[j] *= 2;
                new_row.erase(new_row.begin() + j + 1);
                new_row.push_back(0);
                moved = true;
            }
        }
        for (int j = 0; j < 4; ++j) {
            int val = (j < new_row.size()) ? new_row[j] : 0;
            if (board[i][j] != val) {
                moved = true;
            }
            board[i][j] = val;
        }
    }
    return moved;
}

// 向上移动
bool move_up() {
    bool moved = false;
    // Transpose the board, move left, then transpose back
    std::vector<std::vector<int>> original_board = board;
    std::vector<std::vector<int>> transposed_board(4, std::vector<int>(4));
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            transposed_board[i][j] = board[j][i];
        }
    }
    board = transposed_board;
    moved = move_left();
    transposed_board = board;
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            board[j][i] = transposed_board[i][j];
        }
    }
    return moved;
}

// 向右移动
bool move_right() {
    bool moved = false;
    for (int i = 0; i < 4; ++i) {
        std::vector<int> new_row;
        for (int j = 3; j >= 0; --j) {
            if (board[i][j] != 0) {
                new_row.push_back(board[i][j]);
            }
        }
        for (int j = 0; j < new_row.size() - 1; ++j) {
            if (new_row[j] == new_row[j+1]) {
                new_row[j] *= 2;
                new_row.erase(new_row.begin() + j + 1);
                new_row.push_back(0);
                moved = true;
            }
        }
        for (int j = 0; j < 4; ++j) {
            int val = (j < new_row.size()) ? new_row[j] : 0;
            if (board[i][3-j] != val) {
                moved = true;
            }
            board[i][3-j] = val;
        }
    }
    return moved;
}

// 向下移动
bool move_down() {
    bool moved = false;
    // Transpose the board, move right, then transpose back
    std::vector<std::vector<int>> original_board = board;
    std::vector<std::vector<int>> transposed_board(4, std::vector<int>(4));
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            transposed_board[i][j] = board[j][i];
        }
    }
    board = transposed_board;
    moved = move_right();
    transposed_board = board;
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            board[j][i] = transposed_board[i][j];
        }
    }
    return moved;
}

// 检查游戏是否结束
bool is_game_over() {
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            if (board[i][j] == 0) {
                return false; // 还有空位
            }
            // 检查水平方向是否有可合并的方块
            if (j < 3 && board[i][j] == board[i][j+1]) {
                return false;
            }
            // 检查垂直方向是否有可合并的方块
            if (i < 3 && board[i][j] == board[i+1][j]) {
                return false;
            }
        }
    }
    return true; // 没有空位，也没有可合并的方块
}

// 检查是否胜利
bool check_win() {
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            if (board[i][j] == 2048) {
                return true;
            }
        }
    }
    return false;
}




#include <termios.h>
#include <unistd.h>

// 获取单个字符输入，不带回显，无需回车
int getch() {
    struct termios oldt, newt;
    int ch;
    tcgetattr(STDIN_FILENO, &oldt);
    newt = oldt;
    newt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &newt);
    ch = getchar();
    tcsetattr(STDIN_FILENO, TCSANOW, &oldt);
    return ch;
}

int main() {
    initialize_game();
    while (true) {
        print_board();
        if (check_win()) {
            std::cout << "恭喜你，你赢了！" << std::endl;
            break;
        }
        if (is_game_over()) {
            std::cout << "游戏结束！" << std::endl;
            break;
        }

        std::cout << "请按 W(上), A(左), S(下), D(右) 移动，或按 Q 退出: ";
        char input = getch();
        bool moved = false;
        switch (input) {
            case 'w':
            case 'W':
                moved = move_up();
                break;
            case 'a':
            case 'A':
                moved = move_left();
                break;
            case 's':
            case 'S':
                moved = move_down();
                break;
            case 'd':
            case 'D':
                moved = move_right();
                break;
            case 'q':
            case 'Q':
                std::cout << "退出游戏。" << std::endl;
                return 0;
            default:
                std::cout << "无效输入，请重新输入。" << std::endl;
                continue;
        }
        if (moved) {
            add_random_tile();
        }
    }
    return 0;
}


