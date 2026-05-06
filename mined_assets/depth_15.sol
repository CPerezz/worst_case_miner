// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract WorstCaseERC20 {
    // ERC20 State
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    uint256 public totalSupply;

    // Token metadata - returning constants to save gas
    string public constant name = "WorstCase";
    string public constant symbol = "WORST";
    uint8 public constant decimals = 18;

    constructor() {
        // Mint total supply to deployer
        totalSupply = 1_000_000_000 * 10 ** 18; // 1 billion tokens
        balanceOf[msg.sender] = totalSupply;

        // Set all mined storage keys to 1
        assembly {
            sstore(0x91816bc9a0dfa9765e7d46e735e1e704deae64a3e643e9f31b49636a4ae4a83a, 1)
            sstore(0xf7e19730ca6b7bc73bedc567aefb6acd2250daf23e5753d871b8a5a00a0d32e3, 1)
            sstore(0xf822b46e819a16d5dbf71226662062490b9bff0f70702299f4a60e6a3cf6b181, 1)
            sstore(0xf8f4de14761a98657e053b5875df8fbb0254291355b1c01489871c2e28861f0e, 1)
            sstore(0xf8f975729de3399e82f03f9c7e3868999346c88c56566e15ccec2b311cb32911, 1)
            sstore(0xf8f9cb982de9d2a9076e5173b452267d90e93c817b1d5e27db5bcc291d057e66, 1)
            sstore(0xf8f9c401f35df07a81644739d052cfb99b54405763f7976df9487d40a5567a1f, 1)
            sstore(0xf8f9c42c365c13c75822f2111709773a52f4b3adb053ea1f66fdbd214abb7812, 1)
            sstore(0xf8f9c4213f5f6495b894afd269969b652fe1de654122c55c4da0af3dc5d2eff3, 1)
            sstore(0xf8f9c421157488233d708d1be2bdeb2cf72afcd01ad9a75143c3336c07ee0eca, 1)
            sstore(0xf8f9c42116d187aa6580e9e2482149f515d43aec9fc47dd5eb95d67b1642a67f, 1)
            sstore(0xf8f9c421165325de68a0b4acdb7b8752d5df22f81d73a454b9d9740775bf28f8, 1)
            sstore(0xf8f9c421165d4de51e295d94c25b13c927cb9669189c9dcdba5a3ce6d65cd37b, 1)
            sstore(0xf8f9c421165d4de51e295d94c25b13c927cb9669189c9dcdba5a3ce6d65cd37b, 1)
            sstore(0xf8f9c421165d4f654f5bc1fb34b498a58dcb58bca0c0cc3d5242f91b6201b90d, 1)
        }
    }

    // Minimal ERC20 implementation
    function transfer(address to, uint256 amount) public returns (bool) {
        require(balanceOf[msg.sender] >= amount, "Insufficient balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function approve(address spender, uint256 amount) public returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) public returns (bool) {
        require(balanceOf[from] >= amount, "Insufficient balance");
        require(
            allowance[from][msg.sender] >= amount,
            "Insufficient allowance"
        );

        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        allowance[from][msg.sender] -= amount;

        return true;
    }

    // Attack method - writes to the deepest storage slot
    function attack(uint256 value) external {
        assembly {
            sstore(0xf8f9c421165d4f654f5bc1fb34b498a58dcb58bca0c0cc3d5242f91b6201b90d, value)
        }
    }

    // Optional: getter to verify the deepest slot value
    function getDeepest() external view returns (uint256 value) {
        assembly {
            value := sload(0xf8f9c421165d4f654f5bc1fb34b498a58dcb58bca0c0cc3d5242f91b6201b90d)
        }
    }
}