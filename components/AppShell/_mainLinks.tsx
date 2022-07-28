import React, {useEffect, useState} from 'react';
import {MdAccountBox, MdBookmarks, MdHome} from 'react-icons/md';
import {Avatar, Group, Text, ThemeIcon, UnstyledButton} from '@mantine/core';
import {useRouter} from "next/router";
import styles from './MyAppShell.module.css'
import {appwrite} from "../../utils/appwrite_utils";
import {Models} from "appwrite";
import Link from "next/link";

interface MainLinkProps {
    icon: React.ReactNode;
    color: string;
    label: string;
    link: string;
}

function MainLink({icon, color, label, link}: MainLinkProps) {
    return (
        <Link href={link} passHref={true} prefetch={false}>
            <UnstyledButton
                sx={(theme) => ({
                    display: 'block',
                    width: '100%',
                    height: '70px',
                    padding: theme.spacing.xs,
                    borderRadius: 16,
                    color: theme.colorScheme === 'dark' ? theme.colors.dark[0] : theme.black,

                    '&:hover': {
                        backgroundColor:
                            theme.colorScheme === 'dark' ? theme.colors.dark[5] : theme.colors.gray[2],
                    },
                })}
            >
                <Group noWrap={true}>
                    <ThemeIcon radius={"md"} color={color} size="lg">
                        {icon}
                    </ThemeIcon>

                    <Text size="md" className={styles.mainLinksText}>{label}</Text>
                </Group>
            </UnstyledButton>
        </Link>
    );
}


function UserProfileLink(name: string) {
    return (
        <Link href={"/profile"} passHref={true} prefetch={false}>
            <UnstyledButton
                sx={(theme) => ({
                    display: 'block',
                    width: '100%',
                    height: '70px',
                    padding: theme.spacing.xs,
                    borderRadius: 16,
                    backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[1],
                    color: theme.colorScheme === 'dark' ? theme.colors.dark[0] : theme.black,

                    '&:hover': {
                        backgroundColor:
                            theme.colorScheme === 'dark' ? theme.colors.dark[5] : theme.colors.gray[2],
                    },
                })}
            >
                <Group noWrap={true}>
                    <Avatar
                        src={appwrite.avatars.getInitials(name, 64, 64).toString()}
                        radius={"md"}
                        style={{marginLeft: 6, marginRight: 6}}
                    />
                    <Text size="md" className={styles.mainLinksText}>{name}</Text>
                </Group>
            </UnstyledButton>
        </Link>
    );
}

const data = [
    {icon: <MdHome size={22}/>, color: 'blue', label: 'Acasă', link: 'https://acspa.ro'},
    {icon: <MdBookmarks size={22}/>, color: 'green', label: 'Rezervări', link: '/'},
];

export default function MainLinks() {
    const router = useRouter()
    const [user, setUser] = useState<Models.User<Models.Preferences>>(null)

    useEffect(() => {
        appwrite.account.get()
            .then((account) => setUser(account))
            .catch(() => setUser(null))
    }, [router.asPath]);

    const loginButtonData = {icon: <MdAccountBox size={22}/>, color: 'purple', label: 'Logare', link: '/signin'}

    return <Group noWrap={true}>
        {data.map((link) => (
            <MainLink {...link} key={link.label}/>
        ))}

        {user != null &&
            UserProfileLink(user.name)
        }

        {user == null &&
            <MainLink {...loginButtonData} />
        }
    </Group>;
}
